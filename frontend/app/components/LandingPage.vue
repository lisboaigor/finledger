<!--
    Página institucional servida na raiz do domínio-apex (finledger.com.br sem
    subdomínio). Como o produto é multi-tenant por subdomínio, o CTA central é
    um "composer de endereço": a pessoa digita o slug da empresa e é levada ao
    login de suaempresa.finledger.com.br. Todo o estilo usa os tokens do tema
    (variables/_common.scss), então claro/escuro funcionam sem CSS duplicado.
-->
<script setup lang="ts">
import { Box, FileEdit, LoaderCircle, Moon, Receipt, ShoppingBag, Sun, Truck, Wallet } from '@lucide/vue'
import { Button } from '@/components/ui/button'

const url = useRequestURL()
const config = useRuntimeConfig()
const { toggleDarkMode, isDarkTheme } = useLayout()

const slug = ref('')
const verificando = ref(false)
const erro = ref('')

// Sufixo real do host atual: .finledger.com.br em produção, .localhost:3001 em dev.
const dominioSufixo = computed(() => `.${url.hostname}${url.port ? `:${url.port}` : ''}`)

async function acessar() {
    const s = slug.value
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9-]/g, '')
    if (!s || verificando.value) return

    erro.value = ''
    verificando.value = true
    try {
        // 204 quando o tenant existe e está ativo; 404 caso contrário.
        await $fetch(`${config.public.apiBase}/tenants/${s}/existe`)
        const porta = url.port ? `:${url.port}` : ''
        window.location.href = `${url.protocol}//${s}.${url.hostname}${porta}/login`
    } catch (e) {
        const status = (e as { status?: number; statusCode?: number })
        erro.value =
            (status.status ?? status.statusCode) === 404
                ? `Não encontramos nenhuma empresa em ${s}${dominioSufixo.value}. Confira o endereço.`
                : 'Não foi possível verificar o endereço agora. Tente novamente em instantes.'
        verificando.value = false
    }
}

const modulos = [
    {
        icon: ShoppingBag,
        titulo: 'PDV de balcão',
        texto: 'Terminal de venda rápido, feito para o ritmo do balcão: busca por código, atalhos de teclado e impressão térmica.',
    },
    {
        icon: Box,
        titulo: 'Catálogo e estoque',
        texto: 'Produtos com preço, mínimo e saldo por depósito. Venda confirmada baixa o estoque na hora, sem conferência manual.',
    },
    {
        icon: FileEdit,
        titulo: 'Orçamentos',
        texto: 'Monte, envie e converta orçamentos em venda com um clique — recusas ficam registradas com o motivo.',
    },
    {
        icon: Truck,
        titulo: 'Compras e fornecedores',
        texto: 'Pedidos de compra com recebimento parcial e histórico por fornecedor para negociar melhor.',
    },
    {
        icon: Wallet,
        titulo: 'Financeiro',
        texto: 'Contas a pagar e a receber nascem das operações: cada venda confirmada já vira título no caixa.',
    },
    {
        icon: Receipt,
        titulo: 'Fiscal',
        texto: 'Emissão de nota fiscal encadeada à venda, com status acompanhado dentro do próprio pedido.',
    },
]
</script>

<template>
    <div class="landing">
        <header class="landing-topbar">
            <div class="landing-brand">
                <AppLogoIcon class="landing-brand-icon" />
                <span class="landing-brand-name brand-wordmark">Finledger</span>
            </div>
            <button
                type="button"
                class="landing-theme-toggle"
                :aria-label="isDarkTheme ? 'Usar tema claro' : 'Usar tema escuro'"
                @click="toggleDarkMode"
            >
                <Moon v-if="isDarkTheme" class="size-4" />
                <Sun v-else class="size-4" />
            </button>
        </header>

        <main>
            <!-- ── Hero ─────────────────────────────────────────────────── -->
            <section class="landing-hero">
                <AppLogoIcon class="landing-hero-owl" aria-hidden="true" />
                <p class="landing-eyebrow">ERP para o comércio varejista</p>
                <h1 class="landing-title">
                    Do balcão à nota fiscal,<br />
                    <em>um registro de cada movimento.</em>
                </h1>
                <p class="landing-lead">
                    Venda no PDV, o estoque baixa, o título entra no caixa e a nota sai —
                    encadeados de verdade, não redigitados. Cada empresa opera no seu
                    próprio endereço, com dados isolados.
                </p>

                <form class="landing-composer" @submit.prevent="acessar">
                    <label class="landing-composer-label" for="landing-slug">
                        Já é cliente? Entre pelo endereço da sua empresa
                    </label>
                    <div class="landing-composer-field">
                        <span class="landing-composer-scheme" aria-hidden="true">https://</span>
                        <input
                            id="landing-slug"
                            v-model="slug"
                            type="text"
                            placeholder="suaempresa"
                            autocomplete="off"
                            autocapitalize="none"
                            spellcheck="false"
                        />
                        <span class="landing-composer-suffix">{{ dominioSufixo }}</span>
                        <Button type="submit" :disabled="!slug.trim() || verificando">
                            <LoaderCircle v-if="verificando" class="size-4 animate-spin" />
                            Entrar
                        </Button>
                    </div>
                    <p v-if="erro" class="landing-composer-erro" role="alert">{{ erro }}</p>
                </form>

                <p class="landing-hero-contato">
                    Ainda não tem endereço?
                    <a href="mailto:contato@finledger.com.br">Fale com a gente</a> para criar o da sua empresa.
                </p>
            </section>

            <!-- ── Módulos ──────────────────────────────────────────────── -->
            <section class="landing-modulos" aria-labelledby="landing-modulos-titulo">
                <h2 id="landing-modulos-titulo" class="landing-section-title">
                    Uma operação, um sistema
                </h2>
                <div class="landing-modulos-grid">
                    <article v-for="m in modulos" :key="m.titulo" class="landing-modulo">
                        <component :is="m.icon" class="landing-modulo-icon" aria-hidden="true" />
                        <h3>{{ m.titulo }}</h3>
                        <p>{{ m.texto }}</p>
                    </article>
                </div>
            </section>

            <!-- ── Como funciona ────────────────────────────────────────── -->
            <section class="landing-como" aria-labelledby="landing-como-titulo">
                <h2 id="landing-como-titulo" class="landing-section-title">Como funciona</h2>
                <div class="landing-como-grid">
                    <div class="landing-como-item">
                        <span class="landing-como-tag">endereço próprio</span>
                        <p>
                            Cada empresa vive em <strong>suaempresa</strong>.finledger.com.br —
                            os dados de um cliente nunca se misturam com os de outro.
                        </p>
                    </div>
                    <div class="landing-como-item">
                        <span class="landing-como-tag">papéis e permissões</span>
                        <p>
                            Vendedor, financeiro, estoque, fiscal: cada pessoa da equipe
                            enxerga e faz apenas o que o papel dela permite.
                        </p>
                    </div>
                    <div class="landing-como-item">
                        <span class="landing-como-tag">histórico completo</span>
                        <p>
                            Toda operação é gravada como um evento — dá para auditar quem
                            fez o quê e quando, desde o primeiro dia.
                        </p>
                    </div>
                </div>
            </section>
        </main>

        <footer class="landing-footer">
            <span>© {{ new Date().getFullYear() }} Finledger</span>
            <span>
                Ícone de coruja por
                <a href="https://icons8.com" target="_blank" rel="noopener">Icons8</a>
            </span>
        </footer>
    </div>
</template>

<style scoped>
.landing {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background-color: var(--background);
    color: var(--foreground);
}

.landing-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    max-width: 68rem;
    width: 100%;
    margin: 0 auto;
    padding: 1.25rem 1.5rem;
}

.landing-brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
}

.landing-brand-icon {
    font-size: 2.25rem;
    color: var(--primary);
}

.landing-brand-name {
    --brand-wordmark-base: 1.9rem;
}

.landing-theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.25rem;
    height: 2.25rem;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: transparent;
    color: var(--foreground);
    cursor: pointer;
    transition: background-color 0.15s;
}

.landing-theme-toggle:hover {
    background-color: var(--accent);
}

/* ── Hero ──────────────────────────────────────────────────────────────── */
.landing-hero {
    position: relative;
    overflow: hidden;
    max-width: 68rem;
    width: 100%;
    margin: 0 auto;
    padding: 5rem 1.5rem 4rem;
    text-align: center;
}

/* Coruja-marca-d'água: ambiente, não decoração — é a mesma marca do favicon. */
.landing-hero-owl {
    position: absolute;
    top: -1rem;
    left: 50%;
    transform: translateX(-50%);
    font-size: 22rem;
    color: var(--primary);
    opacity: 0.05;
    pointer-events: none;
}

.landing-eyebrow {
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 0.8rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--primary);
    margin-bottom: 1.25rem;
}

.landing-title {
    font-family: 'Bricolage Grotesque', 'Lato', sans-serif;
    font-size: clamp(2rem, 5vw, 3.25rem);
    font-weight: 800;
    letter-spacing: -0.03em;
    line-height: 1.1;
    margin: 0 0 1.25rem;
}

.landing-title em {
    font-style: normal;
    color: var(--primary);
}

.landing-lead {
    max-width: 38rem;
    margin: 0 auto 2.5rem;
    font-size: 1.125rem;
    line-height: 1.65;
    color: var(--muted-foreground);
}

/* ── Composer de endereço (assinatura da página) ───────────────────────── */
.landing-composer {
    max-width: 34rem;
    margin: 0 auto;
}

.landing-composer-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 600;
    margin-bottom: 0.6rem;
}

.landing-composer-field {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.4rem 0.4rem 0.4rem 0.9rem;
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    background-color: var(--card);
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 0.95rem;
    text-align: left;
}

.landing-composer-field:focus-within {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--ring);
}

.landing-composer-scheme,
.landing-composer-suffix {
    color: var(--muted-foreground);
    white-space: nowrap;
}

.landing-composer-field input {
    flex: 1;
    min-width: 4rem;
    border: none;
    outline: none;
    background: transparent;
    color: var(--primary);
    font: inherit;
    font-weight: 600;
}

.landing-composer-field input::placeholder {
    color: var(--muted-foreground);
    opacity: 0.55;
    font-weight: 400;
}

.landing-composer-erro {
    margin: 0.6rem 0 0;
    font-size: 0.85rem;
    color: var(--destructive);
    text-align: left;
}

.landing-hero-contato {
    margin-top: 1.5rem;
    font-size: 0.875rem;
    color: var(--muted-foreground);
}

.landing-hero-contato a {
    color: var(--primary);
    font-weight: 600;
    text-decoration: none;
}

.landing-hero-contato a:hover {
    text-decoration: underline;
}

/* ── Módulos ───────────────────────────────────────────────────────────── */
.landing-modulos,
.landing-como {
    max-width: 68rem;
    width: 100%;
    margin: 0 auto;
    padding: 3rem 1.5rem;
}

.landing-section-title {
    font-size: 1.5rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    margin: 0 0 2rem;
}

.landing-modulos-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
    gap: 1rem;
}

.landing-modulo {
    padding: 1.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background-color: var(--card);
    transition: border-color 0.15s;
}

.landing-modulo:hover {
    border-color: var(--primary);
}

.landing-modulo-icon {
    font-size: 1.5rem;
    color: var(--primary);
}

.landing-modulo h3 {
    font-size: 1rem;
    font-weight: 700;
    margin: 0.9rem 0 0.4rem;
}

.landing-modulo p {
    font-size: 0.9rem;
    line-height: 1.55;
    color: var(--muted-foreground);
    margin: 0;
}

/* ── Como funciona ─────────────────────────────────────────────────────── */
.landing-como-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
    gap: 2rem;
}

.landing-como-tag {
    display: inline-block;
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 0.75rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--primary);
    margin-bottom: 0.6rem;
}

.landing-como-item p {
    font-size: 0.95rem;
    line-height: 1.6;
    color: var(--muted-foreground);
    margin: 0;
}

/* ── Rodapé ────────────────────────────────────────────────────────────── */
.landing-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    max-width: 68rem;
    width: 100%;
    margin: auto auto 0;
    padding: 2rem 1.5rem;
    border-top: 1px solid var(--border);
    font-size: 0.8rem;
    color: var(--muted-foreground);
}

.landing-footer a {
    color: inherit;
    text-decoration: underline;
}
</style>
