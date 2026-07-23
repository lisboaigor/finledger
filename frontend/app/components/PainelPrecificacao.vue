<script setup lang="ts">
import { Check, ChevronDown, ChevronRight, Clock, ExternalLink, Info, LoaderCircle, Plus, X } from '@lucide/vue'
import type { Elasticidade, PrecoConcorrencia } from '~/models/catalogo'
import {
    listarPrecosConcorrencia,
    obterElasticidade,
    registrarPrecoConcorrencia,
    removerPrecoConcorrencia,
} from '~/models/catalogo'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

/** Painel de apoio à decisão de preço — mostrado no Catálogo (cadastro/edição
 * de produto) e no Estoque (registrar entrada). Toda a linguagem é pensada
 * pra gestor leigo: valores em R$ concretos, percentual como informação
 * secundária, zero jargão contábil. Nada é aplicado automaticamente — o botão
 * "Usar sugestão" apenas emite o valor pro campo de preço, que continua livre. */
const props = withDefaults(
    defineProps<{
        /** Custo direto (produto) em centavos — o que foi pago ao fornecedor. */
        custoCentavos: number
        categoria: string | null
        /** Habilita elasticidade/concorrência/override de custo fixo. */
        produtoId?: string | null
        /** Valor atual do campo "preço de venda" sendo editado (centavos). */
        precoDigitadoCentavos: number
        /** Preço de venda já cadastrado no produto (para "Hoje → Sugerido"). */
        precoVigenteCentavos?: number | null
        sku?: string
        descricao?: string
        /** Habilita o formulário "+ Registrar preço visto" (só no Catálogo). */
        permitirRegistroConcorrencia?: boolean
    }>(),
    {
        produtoId: null,
        precoVigenteCentavos: null,
        sku: '',
        descricao: '',
        permitirRegistroConcorrencia: false,
    },
)

const emit = defineEmits<{ 'usar-sugestao': [precoCentavos: number] }>()

const { formatCentavos } = useFormat()
const { apiFetch } = useApi()
const { sugerirPreco, lucroLiquido, giroDoProduto, garantirCarregado } = useMargens()
const { abrirPesquisaGoogle } = useBuscaConcorrencia()

void garantirCarregado()

const sugestao = computed(() => sugerirPreco(props.custoCentavos, props.categoria, props.produtoId))
const sugestaoValida = computed(() => {
    const s = sugestao.value
    return s && !('invalido' in s) ? s : null
})
const sugestaoInvalida = computed(() => {
    const s = sugestao.value
    return s && 'invalido' in s ? s : null
})

/** Passo a passo do cálculo — fechado por padrão para o cartão ficar leve. */
const explicacaoAberta = ref(false)

/** Custo médio real do estoque divergindo >10% do custo em edição — o valor
 * digitado continua sendo a base (é o custo de reposição corrente), mas o
 * gestor precisa saber que o estoque na prateleira custou diferente. */
const avisoCustoMedio = computed(() => {
    const medio = props.produtoId ? (giroDoProduto(props.produtoId)?.custo_medio_centavos ?? 0) : 0
    if (medio <= 0 || props.custoCentavos <= 0) return null
    const desvio = Math.abs(medio - props.custoCentavos) / props.custoCentavos
    return desvio > 0.1 ? medio : null
})

/** Lucro por unidade no preço que o gestor digitou — atualiza ao vivo. */
const lucro = computed(() => {
    const s = sugestaoValida.value
    if (!s || props.precoDigitadoCentavos <= 0) return null
    return lucroLiquido(s.custoTotalCentavos, props.precoDigitadoCentavos, props.produtoId, props.categoria)
})

// ── Elasticidade + concorrência (por produto, sob demanda) ──────────────────
const elasticidade = ref<Elasticidade | null>(null)
const concorrencia = ref<PrecoConcorrencia[]>([])

watch(
    () => props.produtoId,
    async (id) => {
        elasticidade.value = null
        concorrencia.value = []
        if (!id) return
        // Falha silenciosa: os dois blocos são opcionais no painel.
        const [e, c] = await Promise.allSettled([
            obterElasticidade(apiFetch, id),
            listarPrecosConcorrencia(apiFetch, id),
        ])
        if (e.status === 'fulfilled') elasticidade.value = e.value.elasticidade
        if (c.status === 'fulfilled') concorrencia.value = c.value.precos
    },
    { immediate: true },
)

const fraseElasticidade = computed(() => {
    const e = elasticidade.value
    if (!e) return null
    const dirPreco = e.variacao_preco_pct > 0 ? 'subiu' : 'baixou'
    const dirVendas = e.variacao_vendas_pct > 0 ? 'subiram' : 'caíram'
    return (
        `Da última vez que o preço deste produto ${dirPreco} ` +
        `${Math.abs(e.variacao_preco_pct).toFixed(0)}%, as vendas ${dirVendas} ` +
        `cerca de ${Math.abs(e.variacao_vendas_pct).toFixed(0)}%.`
    )
})

/** Estimativa de impacto se o gestor aplicar a sugestão e ela for um aumento. */
const fraseImpactoAumento = computed(() => {
    const e = elasticidade.value
    const s = sugestaoValida.value
    const vigente = props.precoVigenteCentavos
    if (!e || !s || !vigente || vigente <= 0) return null
    const variacaoPct = (s.precoCentavos / vigente - 1) * 100
    if (variacaoPct < 1) return null
    const quedaEsperada = variacaoPct * e.coeficiente
    if (quedaEsperada >= 0) return null
    return (
        `Com um aumento de ${variacaoPct.toFixed(0)}%, baseado no histórico, ` +
        `é esperada uma queda de ~${Math.abs(quedaEsperada).toFixed(0)}% nas vendas deste produto.`
    )
})

// ── Registro rápido de preço visto (só quando permitido) ────────────────────
const registroVisible = ref(false)
const registroConcorrente = ref('')
const registroPreco = ref<number | null>(null) // em reais na tela
const registrando = ref(false)
const { notifyError, notifySuccess } = useNotify()

async function registrarPrecoVisto() {
    if (!props.produtoId || registroPreco.value == null || registroPreco.value <= 0) return
    registrando.value = true
    try {
        await registrarPrecoConcorrencia(apiFetch, props.produtoId, {
            concorrente: registroConcorrente.value.trim() || null,
            preco_centavos: Math.round(registroPreco.value * 100),
        })
        registroVisible.value = false
        registroConcorrente.value = ''
        registroPreco.value = null
        notifySuccess('Registrado', 'Preço da concorrência anotado.')
        const { precos } = await listarPrecosConcorrencia(apiFetch, props.produtoId)
        concorrencia.value = precos
    } catch {
        notifyError('Não foi possível registrar o preço.')
    } finally {
        registrando.value = false
    }
}

async function removerPrecoVisto(precoId: string) {
    if (!props.produtoId) return
    try {
        await removerPrecoConcorrencia(apiFetch, props.produtoId, precoId)
        concorrencia.value = concorrencia.value.filter((p) => p.id !== precoId)
    } catch {
        notifyError('Não foi possível remover o registro.')
    }
}

function tempoRelativo(iso: string): string {
    const dias = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000)
    if (dias <= 0) return 'hoje'
    if (dias === 1) return 'há 1 dia'
    if (dias < 30) return `há ${dias} dias`
    const meses = Math.floor(dias / 30)
    return meses === 1 ? 'há 1 mês' : `há ${meses} meses`
}
</script>

<template>
    <div v-if="sugestao" class="painel-preco">
        <!-- Configuração inválida (percentuais ≥ 100%) -->
        <MessageBox v-if="sugestaoInvalida" severity="warn">
            {{ sugestaoInvalida.motivo }}
        </MessageBox>

        <template v-else-if="sugestaoValida">
            <!-- Sugestão + Hoje → Sugerido + Usar (preço "de prateleira" arredondado) -->
            <div class="painel-sugestao">
                <span>
                    <template v-if="precoVigenteCentavos && precoVigenteCentavos !== sugestaoValida.precoArredondadoCentavos">
                        Hoje: {{ formatCentavos(precoVigenteCentavos) }} →
                    </template>
                    <strong>Preço sugerido: {{ formatCentavos(sugestaoValida.precoArredondadoCentavos) }}</strong>
                    <span
                        v-if="sugestaoValida.precoArredondadoCentavos !== sugestaoValida.precoCentavos"
                        class="text-muted-foreground text-xs"
                    >
                        (conta exata: {{ formatCentavos(sugestaoValida.precoCentavos) }})
                    </span>
                </span>
                <Button
                    size="sm"
                    variant="outline"
                    @click="emit('usar-sugestao', sugestaoValida.precoArredondadoCentavos)"
                >
                    Usar sugestão
                </Button>
            </div>

            <!-- Custo médio do estoque divergente do custo em edição -->
            <p v-if="avisoCustoMedio" class="painel-linha text-muted-foreground">
                <Info class="mr-1 inline size-3.5" />O estoque deste produto custa em média
                {{ formatCentavos(avisoCustoMedio) }} — diferente do custo usado aqui
                ({{ formatCentavos(custoCentavos) }}). Confira qual reflete a próxima reposição.
            </p>

            <!-- Aviso de giro em destaque, mesmo com o passo a passo fechado -->
            <p
                v-if="sugestaoValida.ajusteGiro && sugestaoValida.ajusteGiro.pontos !== 0"
                class="painel-linha painel-giro"
            >
                <Clock class="mr-1 inline size-3.5" />{{ sugestaoValida.ajusteGiro.motivo }}
            </p>

            <!-- Passo a passo do cálculo -->
            <button type="button" class="painel-toggle" @click="explicacaoAberta = !explicacaoAberta">
                <ChevronDown v-if="explicacaoAberta" class="size-3.5" />
                <ChevronRight v-else class="size-3.5" />
                Como chegamos neste preço?
            </button>
            <ExplicacaoPreco v-if="explicacaoAberta" :sugestao="sugestaoValida" :custo-centavos="custoCentavos" />

            <!-- Lucro ao vivo no preço digitado -->
            <p v-if="lucro && lucro.valorCentavos > 0" class="painel-linha painel-lucro">
                Depois de pagar tudo, sobra {{ formatCentavos(lucro.valorCentavos) }} de lucro por
                unidade — {{ lucro.percentual.toFixed(0) }}% do preço
            </p>
            <MessageBox v-else-if="lucro" severity="error">
                ⚠️ Com esse preço você perde {{ formatCentavos(Math.abs(lucro.valorCentavos)) }} em
                cada unidade vendida
            </MessageBox>

            <!-- Elasticidade (só com histórico suficiente) -->
            <p v-if="fraseElasticidade" class="painel-linha text-muted-foreground">📊 {{ fraseElasticidade }}</p>
            <p v-if="fraseImpactoAumento" class="painel-linha text-muted-foreground">{{ fraseImpactoAumento }}</p>
        </template>

        <!-- Concorrência -->
        <div v-if="produtoId" class="painel-concorrencia">
            <span v-if="concorrencia.length" class="text-muted-foreground">
                Concorrência:
                <template v-for="(p, i) in concorrencia.slice(0, 3)" :key="p.id">
                    <template v-if="i > 0"> · </template>
                    {{ formatCentavos(p.preco_centavos)
                    }}<template v-if="p.concorrente"> ({{ p.concorrente }}, {{ tempoRelativo(p.observado_em) }})</template>
                    <template v-else> ({{ tempoRelativo(p.observado_em) }})</template>
                    <Button
                        v-if="permitirRegistroConcorrencia"
                        variant="ghost"
                        size="icon-sm"
                        class="painel-remover-preco"
                        :aria-label="'Remover preço registrado'"
                        @click="removerPrecoVisto(p.id)"
                    >
                        <X class="size-3.5" />
                    </Button>
                </template>
            </span>
            <span v-else class="text-muted-foreground">Compare com o preço da concorrência antes de confirmar.</span>
            <span class="flex items-center shrink-0">
                <Button
                    v-if="permitirRegistroConcorrencia"
                    variant="ghost"
                    size="sm"
                    @click="registroVisible = !registroVisible"
                >
                    <Plus class="size-4" />
                    Registrar preço visto
                </Button>
                <Button variant="ghost" size="sm" @click="abrirPesquisaGoogle(sku, descricao)">
                    <ExternalLink class="size-4" />
                    Pesquisar no Google
                </Button>
            </span>
        </div>

        <!-- Formulário rápido de registro de preço visto -->
        <div v-if="registroVisible && permitirRegistroConcorrencia" class="painel-registro">
            <Input v-model="registroConcorrente" placeholder="Loja (opcional)" class="flex-1 h-8" />
            <InputMoney v-model="registroPreco" placeholder="Preço visto" class="w-36 h-8" />
            <Button size="icon-sm" :disabled="!registroPreco || registrando" aria-label="Salvar preço visto" @click="registrarPrecoVisto">
                <LoaderCircle v-if="registrando" class="size-4 animate-spin" />
                <Check v-else class="size-4" />
            </Button>
        </div>
    </div>
</template>

<style scoped>
.painel-preco {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    margin-top: 0.5rem;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-left: 3px solid var(--primary);
    border-radius: 6px;
    background: var(--muted);
    font-size: 0.85rem;
}

.painel-linha {
    margin: 0;
}

.painel-sugestao {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.95rem;
}

.painel-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: none;
    border: none;
    padding: 0;
    font-size: 0.85rem;
    color: var(--muted-foreground);
    cursor: pointer;
    width: fit-content;
}

.painel-lucro {
    color: var(--color-emerald-600, #16a34a);
    font-weight: 600;
}

.painel-concorrencia {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    border-top: 1px dashed var(--border);
    padding-top: 0.5rem;
    margin-top: 0.25rem;
}

.painel-remover-preco {
    width: 1.25rem !important;
    height: 1.25rem !important;
    vertical-align: middle;
}

.painel-registro {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}
</style>
