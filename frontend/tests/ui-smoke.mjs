/**
 * Smoke test de UI (Playwright) das features de precificação, lixeira, score
 * de saúde e análise de preços — roda contra o ambiente local de dev:
 *   just db && just back && just front   (tenant demo, admin/admin)
 *   node frontend/tests/ui-smoke.mjs     (requer `npm i playwright` disponível)
 *
 * O passo da lixeira usa `docker exec` no Postgres local para envelhecer uma
 * venda e rodar a varredura de arquivamento — mesmo caminho do job.
 */
import { chromium } from 'playwright'
import { execSync } from 'node:child_process'

const BASE = 'http://demo.localhost:3001'
const PSQL = 'docker exec finledger-postgres-1 psql -U postgres -d finledger -At -c'
const TENANT_DEMO = 'a0000000-0000-0000-0000-000000000001'

let falhas = 0
function check(nome, cond) {
    console.log(`${cond ? 'ok ' : 'FALHOU'} ${nome}`)
    if (!cond) falhas++
}

const browser = await chromium.launch()
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

// ── Login ────────────────────────────────────────────────────────────────────
await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' })
await page.fill('#tenant-usuario', 'admin')
await page.fill('#tenant-senha input', 'admin')
await page.click('button[type=submit]')
await page.waitForURL(`${BASE}/`, { timeout: 15000 })

// ── 1. Dashboard: score de saúde com componentes ─────────────────────────────
await page.waitForLoadState('networkidle')
await page.waitForTimeout(1500)
check('score de saúde visível no dashboard', await page.locator('.saude-anel').isVisible())
check(
    'score com componentes detalhados',
    (await page.locator('.saude-componentes li').count()) >= 3,
)
check('sino de notificações no topbar', await page.locator('.layout-topbar .pi-bell').isVisible())

// ── 2. Configurações: custo fixo discriminado + prazo da lixeira ─────────────
await page.goto(`${BASE}/configuracoes`, { waitUntil: 'networkidle' })
await page.waitForTimeout(1500)
await page.fill('#custo-nome', 'Energia (teste UI)')
// InputNumber do PrimeVue só atualiza o modelo com eventos de teclado reais.
await page.locator('#custo-valor input').pressSequentially('100')
await page.keyboard.press('Tab')
await page.click('button:has-text("Salvar custo")')
await page.waitForTimeout(1200)
check(
    'custo fixo aparece na tabela',
    await page.locator('td:has-text("Energia (teste UI)")').isVisible(),
)
check(
    'campo de total fica somente leitura com itens',
    await page.locator('#custos-fixos input').isDisabled(),
)
// prazo da lixeira
await page.locator('#arquivamento-dias input').pressSequentially('30')
await page.keyboard.press('Tab')
await page.locator('fieldset:has-text("Limpeza automática") button:has-text("Salvar")').click()
await page.waitForTimeout(1200)
await page.reload({ waitUntil: 'networkidle' })
await page.waitForTimeout(1500)
check(
    'prazo de arquivamento persistido',
    (await page.locator('#arquivamento-dias input').inputValue()).includes('30'),
)
// limpa o custo de teste
await page.locator('tr:has-text("Energia (teste UI)") button .pi-trash').click()
await page.waitForTimeout(1000)

// ── 3. Lixeira de vendas: arquivar via rotina, restaurar via UI ──────────────
execSync(
    `${PSQL} "UPDATE proj_vendas SET atualizado_em = NOW() - INTERVAL '60 days' WHERE tenant_id = '${TENANT_DEMO}' AND status = 'iniciada'"`,
)
execSync(`${PSQL} "SELECT executar_arquivamento()"`)
await page.goto(`${BASE}/vendas`, { waitUntil: 'networkidle' })
await page.waitForTimeout(1200)
await page.click('button:has-text("Lixeira")')
await page.waitForTimeout(1500)
const linhasLixeira = await page.locator('.p-dialog tbody tr:has(button:has-text("Restaurar"))').count()
check('lixeira de vendas lista arquivadas', linhasLixeira >= 1)
if (linhasLixeira) {
    await page.locator('.p-dialog button:has-text("Restaurar")').first().click()
    await page.waitForTimeout(1200)
    check(
        'restauração remove da lixeira',
        (await page.locator('.p-dialog tbody tr:has(button:has-text("Restaurar"))').count()) ===
            linhasLixeira - 1,
    )
}
await page.keyboard.press('Escape')

// ── 4. Análises → Preços e Margens: busca filtra a tabela ────────────────────
await page.goto(`${BASE}/analises`, { waitUntil: 'networkidle' })
await page.waitForTimeout(2500)
await page.click('button:has-text("Preços e Margens"), a:has-text("Preços e Margens")')
await page.waitForTimeout(1500)
const antes = await page.locator('.p-tabpanel:visible tbody tr').count()
await page.fill('.p-tabpanel:visible input[placeholder*="Buscar por SKU"]', 'AMORT')
await page.waitForTimeout(800)
const depois = await page.locator('.p-tabpanel:visible tbody tr').count()
check('busca filtra a análise de preços', depois > 0 && depois < antes)
check(
    'linhas filtradas contêm o termo',
    (await page.locator('.p-tabpanel:visible tbody tr').first().innerText()).includes('AMORT'),
)
// passo a passo "Por quê?"
await page.locator('.p-tabpanel:visible button:has-text("Por quê?")').first().click()
await page.waitForTimeout(800)
check('dialog "Por quê?" mostra o passo a passo', await page.locator('.explicacao-preco').isVisible())

await browser.close()
console.log(falhas ? `\n${falhas} verificações falharam` : '\ntodos os checks de UI passaram')
process.exit(falhas ? 1 : 0)
