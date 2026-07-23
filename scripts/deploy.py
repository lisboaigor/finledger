#!/usr/bin/env python3
"""Deploy do Finledger na VPS de produção (Hostinger).

O que faz, em ordem:
  1. Confere que a árvore git está limpa (deploya exatamente o HEAD commitado).
  2. Envia a árvore commitada via `git archive` (só arquivos versionados — sem
     node_modules, target, .nuxt nem artefatos stray) para um staging na VPS.
  3. Sincroniza staging → produção com rsync SEM --delete, protegendo o
     .env.prod (segredos, exclusivo de prod).
  4. Remove cirurgicamente só os arquivos que sumiram desde o último deploy
     (calculado a partir do marcador .deployed_commit no servidor) — nunca
     apaga diretórios de prod desconhecidos.
  5. Aplica todas as migrações SQL na base viva (idempotentes) ANTES do rebuild.
  6. Rebuild + restart via docker compose (build primeiro: se falhar, a versão
     antiga continua no ar — zero downtime numa falha de build).
  7. Grava o commit deployado e faz um health check.

Config (usuário/IP da VPS) NÃO fica no código: defina VPS_HOST no ambiente ou
em scripts/.deploy.env (gitignorado). Veja scripts/.deploy.env.example.

Uso:
  ./scripts/deploy.py                 # pede confirmação (produção, cliente real)
  FORCE=1 ./scripts/deploy.py         # sem confirmação (ex.: automação)
  ALLOW_DIRTY=1 ./scripts/deploy.py   # permite árvore suja (deploya só o commitado)
"""
from __future__ import annotations

import os
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
SSH_OPTS = ["-o", "BatchMode=yes", "-o", "ServerAliveInterval=30", "-o", "ServerAliveCountMax=10"]
MARKER = ".deployed_commit"

# ── Cores / log ───────────────────────────────────────────────────────────────
_C = {"info": "\033[1;36m", "ok": "\033[1;32m", "warn": "\033[1;33m", "err": "\033[1;31m", "off": "\033[0m"}


def log(msg: str) -> None:
    print(f"\n{_C['info']}▶ {msg}{_C['off']}")


def ok(msg: str) -> None:
    print(f"{_C['ok']}✓ {msg}{_C['off']}")


def warn(msg: str) -> None:
    print(f"{_C['warn']}! {msg}{_C['off']}")


def die(msg: str) -> "None":
    print(f"\n{_C['err']}✗ {msg}{_C['off']}", file=sys.stderr)
    sys.exit(1)


# ── Config: ambiente > scripts/.deploy.env > padrão ───────────────────────────
def carregar_env_file(path: Path) -> dict[str, str]:
    vals: dict[str, str] = {}
    if not path.exists():
        return vals
    for linha in path.read_text().splitlines():
        linha = linha.strip()
        if not linha or linha.startswith("#") or "=" not in linha:
            continue
        chave, valor = linha.split("=", 1)
        vals[chave.strip()] = valor.strip().strip('"').strip("'")
    return vals


_FILE_CFG = carregar_env_file(SCRIPT_DIR / ".deploy.env")


def cfg(chave: str, padrao: str | None = None, obrigatorio: bool = False) -> str:
    valor = os.environ.get(chave) or _FILE_CFG.get(chave) or padrao
    if obrigatorio and not valor:
        die(f"defina {chave} no ambiente ou em scripts/.deploy.env — veja scripts/.deploy.env.example")
    return valor or ""


def flag(chave: str) -> bool:
    return (os.environ.get(chave) or _FILE_CFG.get(chave) or "0") == "1"


# ── Execução de comandos ──────────────────────────────────────────────────────
def run(cmd: list[str], *, capture: bool = False, stdin=None, check: bool = True) -> str:
    res = subprocess.run(
        cmd,
        stdin=stdin,
        stdout=subprocess.PIPE if capture else None,
        text=True,
        check=False,
    )
    if check and res.returncode != 0:
        die(f"comando falhou ({res.returncode}): {' '.join(cmd)}")
    return (res.stdout or "").strip() if capture else ""


def git(*args: str, capture: bool = True, check: bool = True) -> str:
    return run(["git", *args], capture=capture, check=check)


def commit_existe(sha: str) -> bool:
    """True se o commit existe neste clone (para calcular deleções com segurança)."""
    return subprocess.run(
        ["git", "cat-file", "-e", f"{sha}^{{commit}}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode == 0


def main() -> None:
    vps_host = cfg("VPS_HOST", obrigatorio=True)
    remote_dir = cfg("REMOTE_DIR", "/opt/finledger")
    staging_dir = cfg("STAGING_DIR", "/opt/finledger-deploy")
    compose_file = cfg("COMPOSE_FILE", "docker-compose.prod.yml")
    env_file = cfg("ENV_FILE", ".env.prod")
    pg_container = cfg("PG_CONTAINER", "finledger-postgres-1")
    health_url = cfg("HEALTH_URL", "https://finledger.com.br/")

    def rssh(remote_cmd: str, *, capture: bool = False, check: bool = True) -> str:
        return run(["ssh", *SSH_OPTS, vps_host, remote_cmd], capture=capture, check=check)

    # Raiz do repo, independentemente de onde o script é chamado.
    os.chdir(git("-C", str(SCRIPT_DIR), "rev-parse", "--show-toplevel"))

    # ── 1. Preflight ──────────────────────────────────────────────────────────
    head_sha = git("rev-parse", "HEAD")
    head_short = git("rev-parse", "--short", "HEAD")
    branch = git("rev-parse", "--abbrev-ref", "HEAD")

    if not flag("ALLOW_DIRTY"):
        sujo = (
            git("status", "--porcelain") != ""
        )
        if sujo:
            die("árvore git suja — commit/stash antes (ou ALLOW_DIRTY=1). O deploy usa só o commitado.")

    log(f"Deploy do commit {head_short} ({branch}) → {vps_host}:{remote_dir}")
    rssh("true")  # valida a conexão (die se falhar)

    remote_sha = rssh(f"cat {remote_dir}/{MARKER} 2>/dev/null || true", capture=True)
    if remote_sha and commit_existe(remote_sha):
        print(f"  Em produção agora: {git('rev-parse', '--short', remote_sha)}")
        n = len(git("diff", "--name-only", remote_sha, "HEAD").splitlines())
        print(f"  Mudanças neste deploy: {n} arquivo(s)")
    elif remote_sha:
        warn(f"commit em produção ({remote_sha[:12]}) não existe neste clone — deleções serão puladas")
        remote_sha = ""
    else:
        warn("sem marcador de deploy anterior — deleções de arquivos serão puladas neste deploy")

    # ── 2. Confirmação (produção com cliente real) ────────────────────────────
    if not flag("FORCE") and sys.stdin.isatty():
        resp = input(f"\n{_C['warn']}Confirma o deploy em PRODUÇÃO? [y/N] {_C['off']}")
        if resp.strip().lower() != "y":
            die("cancelado")

    # ── 3. Envia a árvore commitada para o staging ────────────────────────────
    log(f"Enviando árvore do commit para {staging_dir} (staging)")
    extrair = f"rm -rf {staging_dir} && mkdir -p {staging_dir} && tar -x -C {staging_dir}"
    p_archive = subprocess.Popen(["git", "archive", "--format=tar", "HEAD"], stdout=subprocess.PIPE)
    p_ssh = subprocess.Popen(["ssh", *SSH_OPTS, vps_host, extrair], stdin=p_archive.stdout)
    if p_archive.stdout:
        p_archive.stdout.close()
    p_ssh.communicate()
    p_archive.wait()
    if p_ssh.returncode != 0 or p_archive.returncode != 0:
        die("falha ao enviar a árvore para o staging")
    n_versionados = len(git("ls-files").splitlines())
    ok(f"árvore enviada ({n_versionados} arquivos versionados)")

    # ── 4. Sincroniza staging → produção (sem --delete; protege .env.prod) ────
    log(f"Sincronizando para {remote_dir} (preservando {env_file})")
    md5_antes = rssh(f"md5sum {remote_dir}/{env_file} 2>/dev/null | cut -d' ' -f1 || echo ausente", capture=True)
    rssh(f"rsync -a --exclude='{env_file}' {staging_dir}/ {remote_dir}/")
    md5_depois = rssh(f"md5sum {remote_dir}/{env_file} 2>/dev/null | cut -d' ' -f1 || echo ausente", capture=True)
    if md5_antes != md5_depois:
        die(f"{env_file} foi alterado — abortando (não deveria acontecer)")
    ok(f"código sincronizado; {env_file} intacto")

    # ── 5. Remove só os arquivos apagados desde o último deploy ───────────────
    # Inclui o lado antigo de renames (git diff -M classifica como R, não D —
    # sem isso o caminho velho sobrevive no servidor ao lado do novo e quebra
    # o build, ex.: E0761 de módulo duplicado).
    if remote_sha:
        deletados: list[str] = []
        for linha in git("diff", "--diff-filter=DR", "--name-status", "-M", remote_sha, "HEAD").splitlines():
            if not linha.strip():
                continue
            status, *paths = linha.split("\t")
            caminho_antigo = paths[0]
            if status.startswith("R"):
                caminho_antigo = paths[0]  # lado antigo do rename; paths[1] é o novo
            if caminho_antigo and caminho_antigo != env_file:
                deletados.append(caminho_antigo)
        if deletados:
            log(f"Removendo {len(deletados)} arquivo(s) apagado(s) neste release")
            for f in deletados:
                rssh(f"rm -f -- {remote_dir}/{f}")
            ok("arquivos obsoletos removidos")

    # ── 6. Migrações na base viva (idempotentes), ANTES do rebuild ────────────
    log("Aplicando migrações SQL em produção (idempotentes)")
    migrar = (
        f"set -e; for f in $(ls {remote_dir}/docker/postgres/migrations/*.sql | sort); do "
        f"docker exec -i {pg_container} psql -v ON_ERROR_STOP=1 -U postgres -d finledger < \"$f\" >/dev/null; done"
    )
    rssh(migrar)
    ok("migrações aplicadas")

    # ── 6b. Schema analítico bi.sql (idempotente), ANTES do rebuild ───────────
    # bi.sql NÃO está em migrations/ (é reaplicável por natureza: CREATE OR
    # REPLACE + ADD COLUMN IF NOT EXISTS). Sem reaplicá-lo, o schema `bi` diverge
    # do código a cada release que o altera — foi a causa do dashboard vazio
    # (colunas/funções ausentes → 500 nos /bi/*). Reaplicar aqui mantém o
    # warehouse em sincronia com o binário. O ETL do próprio backend repopula os
    # fatos no próximo ciclo.
    log("Reaplicando o schema analítico bi.sql (idempotente)")
    rssh(
        f"docker exec -i {pg_container} psql -v ON_ERROR_STOP=1 -U postgres -d finledger "
        f"< {remote_dir}/docker/postgres/bi.sql >/dev/null"
    )
    ok("bi.sql aplicado")

    # ── 7. Rebuild + restart (build antes do recreate) ────────────────────────
    log("Rebuild + restart (docker compose up -d --build) — pode levar alguns minutos")
    rssh(f"cd {remote_dir} && docker compose -f {compose_file} --env-file {env_file} up -d --build")

    # ── 8. Marcador + health check ────────────────────────────────────────────
    rssh(f"printf '%s' '{head_sha}' > {remote_dir}/{MARKER}")
    log("Status dos containers")
    rssh(f"cd {remote_dir} && docker compose -f {compose_file} --env-file {env_file} ps")

    log(f"Health check: {health_url}")
    code = 0
    try:
        with urllib.request.urlopen(urllib.request.Request(health_url, method="GET"), timeout=20) as r:
            code = r.status
    except urllib.error.HTTPError as e:
        code = e.code
    except Exception:
        code = 0
    if code in (200, 301, 302, 308):
        ok(f"aplicação respondendo (HTTP {code})")
    else:
        warn(f"health check retornou HTTP {code} — verifique: ssh {vps_host} 'cd {remote_dir} && docker compose logs -f'")

    print(f"\n{_C['ok']}✓ Deploy do commit {head_short} concluído.{_C['off']}")


if __name__ == "__main__":
    main()
