<script setup lang="ts">
import type { SugestaoPreco } from '~/composables/useMargens'

/** Passo a passo do preço sugerido — os números e argumentos que levaram ao
 * valor. Reutilizado pelo PainelPrecificacao (Catálogo/Estoque) e pela análise
 * de preços do BI; todo o cálculo vem pronto de useMargens (nada é refeito
 * aqui, só apresentado). */
const props = defineProps<{
    sugestao: SugestaoPreco
    /** Custo direto (o que foi pago ao fornecedor), em centavos. */
    custoCentavos: number
}>()

const { formatCentavos } = useFormat()

const fmtPct = (pct: number) => `${Number(pct.toFixed(2))}%`

const origemMargemLabel = computed(() => {
    if (props.sugestao.origemMargem === 'produto') return 'definida neste produto'
    if (props.sugestao.origemMargem === 'categoria') return 'definida na categoria'
    return 'padrão da loja'
})

/** Custo DIRETO por unidade (embalagem, frete-compra) em R$ — só quando
 * definido explicitamente no produto ou na categoria (override). */
const explicacaoCustoDireto = computed(() => {
    const s = props.sugestao
    if (s.custoDiretoUnitarioCentavos <= 0) return null
    if (s.origemCustoDireto === 'produto') return { rotulo: 'valor definido neste produto' }
    if (s.origemCustoDireto === 'categoria') return { rotulo: 'valor definido na categoria' }
    return { rotulo: '' }
})

const descontosPct = computed(() => props.sugestao.descontos.reduce((s, d) => s + d.pct, 0))

/** Decomposição do preço sugerido em R$ — o argumento final: para onde vai
 * cada parte do preço e o que sobra de lucro. */
const decomposicao = computed(() => {
    const s = props.sugestao
    const partes = [
        { nome: 'Pagar o produto', valor: props.custoCentavos },
        ...(s.custoDiretoUnitarioCentavos > 0
            ? [{ nome: 'Custo direto por unidade', valor: s.custoDiretoUnitarioCentavos }]
            : []),
        ...s.descontos.map((d) => ({ nome: d.nome, valor: Math.round((s.precoCentavos * d.pct) / 100) })),
    ]
    const lucro = s.precoCentavos - partes.reduce((sum, p) => sum + p.valor, 0)
    return { partes, lucro }
})

const numeroPasso = computed(() => {
    // Passos exibidos variam (custo direto e giro são opcionais) — numeração dinâmica.
    let n = 1
    return {
        custo: n++,
        direto: props.sugestao.custoDiretoUnitarioCentavos > 0 ? n++ : null,
        descontos: props.sugestao.descontos.length ? n++ : null,
        margem: n++,
        giro: props.sugestao.ajusteGiro ? n++ : null,
        conta: n,
    }
})
</script>

<template>
    <ol class="explicacao-preco">
        <!-- Custo direto -->
        <li>
            <span class="passo-numero">{{ numeroPasso.custo }}</span>
            <span>
                Você pagou <strong>{{ formatCentavos(custoCentavos) }}</strong> pelo produto.
            </span>
        </li>

        <!-- Custo direto por unidade em R$ (override por produto/categoria) -->
        <li v-if="numeroPasso.direto && explicacaoCustoDireto">
            <span class="passo-numero">{{ numeroPasso.direto }}</span>
            <span>
                Cada unidade tem
                <strong>{{ formatCentavos(sugestao.custoDiretoUnitarioCentavos) }}</strong> de custo direto
                (embalagem, frete)
                <template v-if="explicacaoCustoDireto.rotulo">({{ explicacaoCustoDireto.rotulo }})</template>
                → custo total de <strong>{{ formatCentavos(sugestao.custoTotalCentavos) }}</strong
                >.
            </span>
        </li>

        <!-- Descontos sobre o preço -->
        <li v-if="numeroPasso.descontos">
            <span class="passo-numero">{{ numeroPasso.descontos }}</span>
            <span>
                Do preço final saem
                <template v-for="(d, i) in sugestao.descontos" :key="d.nome">
                    <template v-if="i > 0"> + </template>
                    {{ d.nome.toLowerCase() }} ({{ fmtPct(d.pct) }})
                </template>
                = <strong>{{ fmtPct(descontosPct) }}</strong> do preço.
            </span>
        </li>

        <!-- Margem desejada -->
        <li>
            <span class="passo-numero">{{ numeroPasso.margem }}</span>
            <span>
                Você quer que sobre <strong>{{ fmtPct(sugestao.margemBasePct) }}</strong> do preço como
                lucro ({{ origemMargemLabel }}).
            </span>
        </li>

        <!-- Ajuste por giro -->
        <li v-if="numeroPasso.giro && sugestao.ajusteGiro">
            <span class="passo-numero">{{ numeroPasso.giro }}</span>
            <span>
                {{ sugestao.ajusteGiro.motivo }}<template v-if="sugestao.ajusteGiro.pontos !== 0">
                    → margem ajustada de {{ fmtPct(sugestao.margemBasePct) }} para
                    <strong>{{ fmtPct(sugestao.margemPct) }}</strong></template
                >.
            </span>
        </li>

        <!-- Conta final -->
        <li>
            <span class="passo-numero">{{ numeroPasso.conta }}</span>
            <span>
                Conta final: {{ formatCentavos(sugestao.custoTotalCentavos) }} ÷ (100% −
                {{ fmtPct(descontosPct) }} − {{ fmtPct(sugestao.margemPct) }}) =
                {{ formatCentavos(sugestao.precoCentavos) }}<template
                    v-if="sugestao.precoArredondadoCentavos !== sugestao.precoCentavos"
                >, arredondado para cima ao preço "de prateleira":
                    <strong class="text-primary">{{ formatCentavos(sugestao.precoArredondadoCentavos) }}</strong>
                    (nunca para baixo — o arredondamento não corrói a margem)</template
                ><strong v-else class="text-primary"> — {{ formatCentavos(sugestao.precoCentavos) }}</strong>
            </span>
        </li>
    </ol>

    <!-- Argumento em R$: para onde vai cada parte do preço sugerido -->
    <div class="decomposicao">
        <p class="text-muted-foreground mb-1">Em cada venda de {{ formatCentavos(sugestao.precoCentavos) }}:</p>
        <ul>
            <li v-for="p in decomposicao.partes" :key="p.nome" class="flex justify-between gap-4">
                <span class="text-muted-foreground">{{ p.nome }}</span>
                <span>{{ formatCentavos(p.valor) }}</span>
            </li>
            <li class="flex justify-between gap-4 lucro">
                <span>Sobra de lucro</span>
                <span>{{ formatCentavos(decomposicao.lucro) }} ({{ fmtPct(sugestao.margemPct) }})</span>
            </li>
        </ul>
    </div>
</template>

<style scoped>
.explicacao-preco {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 0.85rem;
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    overflow: hidden;
}

.explicacao-preco li {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    padding: 0.625rem 0.75rem;
}

.explicacao-preco li + li {
    border-top: 1px solid var(--border);
}

.explicacao-preco li:nth-child(odd) {
    background: var(--muted);
}

.passo-numero {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 9999px;
    background: var(--color-emerald-100, #d1fae5);
    color: var(--color-emerald-700, #047857);
    font-size: 0.7rem;
    font-weight: 700;
}

.app-dark .passo-numero {
    background: color-mix(in srgb, var(--primary) 18%, transparent);
    color: var(--color-emerald-300);
}

.decomposicao {
    margin-top: 0.75rem;
    padding: 0.75rem 0.875rem;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    box-shadow: 0 1px 3px 0 rgb(0 0 0 / 0.08), 0 1px 2px -1px rgb(0 0 0 / 0.08);
    font-size: 0.8rem;
}

.decomposicao ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
}

.decomposicao .lucro {
    border-top: 1px solid var(--border);
    margin-top: 0.25rem;
    padding-top: 0.25rem;
    font-weight: 600;
    color: var(--color-emerald-600, #16a34a);
}
</style>
