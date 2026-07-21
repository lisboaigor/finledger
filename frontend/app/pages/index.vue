<script setup lang="ts">
// A raiz muda de papel conforme o host: sem subdomínio (apex) é a página
// institucional (sem o layout autenticado — daí layout: false + <NuxtLayout>
// manual no ramo do dashboard); com subdomínio de tenant é o dashboard.
definePageMeta({ layout: false })

import type { Component } from 'vue'
import {
    AlertCircle,
    ArrowDownLeft,
    ArrowRight,
    ArrowUpRight,
    Check,
    ChevronDown,
    ChevronUp,
    FileCheck,
    FileText,
    Flag,
    Percent,
    ShoppingCart,
    Tag,
    TrendingUp,
    Users,
    Wallet,
    X,
    Zap,
} from '@lucide/vue'
import { CurveType } from '@unovis/ts'
import { VisArea, VisAxis, VisGroupedBar, VisLine, VisXYContainer } from '@unovis/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import {
    ChartContainer,
    ChartCrosshair,
    ChartTooltip,
    ChartTooltipContent,
    componentToString,
} from '@/components/ui/chart'
import { Skeleton } from '@/components/ui/skeleton'

const { isBackoffice, tenantSlug } = useSubdomain()
const isLanding = computed(() => !isBackoffice.value && !tenantSlug.value)

// Subdomínio de backoffice não tem dashboard de tenant — vai para /tenants
if (import.meta.client && isBackoffice.value) {
    await navigateTo('/tenants')
}

const vm = useDashboardViewModel()
const bi = useBiViewModel()
const notif = useNotificacoes()
const { formatCentavos } = useFormat()

onMounted(() => {
    if (!isLanding.value) {
        vm.carregar()
        bi.carregar()
        notif.carregar()
    }
})

// Dashboard calmo: só as 3 recomendações mais importantes; o restante fica no
// sino do topbar (e no "ver todas" abaixo).
const MOSTRAR_FECHADO = 3
const alertasExpandidos = ref(false)
const alertasVisiveis = computed(() =>
    alertasExpandidos.value ? notif.alertas : notif.alertas.slice(0, MOSTRAR_FECHADO),
)

const tomClasses: Record<string, string> = {
    bom: 'text-green-600 dark:text-green-400',
    atencao: 'text-orange-500',
    critico: 'text-red-500',
    neutro: 'text-muted-foreground',
}

// Os viewmodels (não editados nesta migração) ainda emitem classes de ícone
// PrimeIcons (`pi pi-xxx`) — a fonte pi não é mais carregada, então mapeamos
// pra ícones lucide aqui na view.
const ICONE_POR_CLASSE_PI: Record<string, Component> = {
    'pi pi-chart-line': TrendingUp,
    'pi pi-arrow-down-left': ArrowDownLeft,
    'pi pi-arrow-up-right': ArrowUpRight,
    'pi pi-shopping-cart': ShoppingCart,
    'pi pi-file': FileText,
    'pi pi-tag': Tag,
    'pi pi-users': Users,
    'pi pi-file-check': FileCheck,
    'pi pi-wallet': Wallet,
    'pi pi-exclamation-circle': AlertCircle,
    'pi pi-percentage': Percent,
}
function iconePi(classe: string): Component {
    return ICONE_POR_CLASSE_PI[classe] ?? Tag
}
</script>

<template>
    <LandingPage v-if="isLanding" />
    <NuxtLayout v-else name="default">
    <div>
        <div class="mb-6">
            <h1 class="text-2xl font-semibold">Olá, {{ vm.username || 'usuário' }} 👋</h1>
            <p class="text-muted-foreground">Resumo operacional do seu negócio.</p>
        </div>

        <!-- O que fazer hoje: as recomendações mais importantes, compactas.
             A lista completa vive no sino do topbar — nada de mural de avisos. -->
        <Card v-if="notif.alertas.length" class="mb-4">
            <CardContent>
                <div class="flex items-center gap-2 mb-3">
                    <Zap class="text-primary size-5" />
                    <span class="text-lg font-semibold">O que fazer hoje</span>
                    <span class="rounded-full bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-800 dark:bg-amber-500/15 dark:text-amber-300">
                        {{ notif.alertas.length }}
                    </span>
                </div>
                <ul class="alerta-lista">
                    <li v-for="alerta in alertasVisiveis" :key="alerta.alerta_id" class="alerta-item">
                        <component :is="notif.iconeAlerta(alerta.codigo)" class="alerta-icone size-4" />
                        <div class="flex-1 min-w-0">
                            <p class="font-medium text-sm leading-snug">{{ alerta.titulo }}</p>
                            <p class="text-xs text-muted-foreground leading-snug">{{ alerta.mensagem }}</p>
                        </div>
                        <div class="flex items-center shrink-0">
                            <NuxtLink :to="alerta.link">
                                <Button variant="ghost" size="icon-sm" title="Abrir">
                                    <ArrowRight class="size-4" />
                                </Button>
                            </NuxtLink>
                            <Button
                                variant="ghost"
                                size="icon-sm"
                                class="text-emerald-600"
                                title="Marcar como resolvido"
                                :disabled="notif.enviandoFeedback === alerta.alerta_id"
                                @click="notif.feedback(alerta, 'resolvido')"
                            >
                                <Check class="size-4" />
                            </Button>
                            <Button
                                variant="ghost"
                                size="icon-sm"
                                title="Ignorar por 30 dias"
                                :disabled="notif.enviandoFeedback === alerta.alerta_id"
                                @click="notif.feedback(alerta, 'ignorado')"
                            >
                                <X class="size-4" />
                            </Button>
                        </div>
                    </li>
                </ul>
                <Button
                    v-if="notif.alertas.length > 3"
                    variant="ghost"
                    size="sm"
                    class="mt-2"
                    @click="alertasExpandidos = !alertasExpandidos"
                >
                    <ChevronUp v-if="alertasExpandidos" class="size-4" />
                    <ChevronDown v-else class="size-4" />
                    {{ alertasExpandidos ? 'Mostrar menos' : `Ver todas as ${notif.alertas.length}` }}
                </Button>
            </CardContent>
        </Card>

        <!-- Score de saúde: nota 0–100 composta pelas métricas do BI, com o porquê -->
        <Card v-if="bi.saude?.score != null && bi.tomSaude" class="mb-4">
            <CardContent>
                <div class="flex flex-col sm:flex-row gap-6 items-center">
                    <div class="flex flex-col items-center shrink-0">
                        <div class="saude-anel" :class="`saude-${bi.tomSaude.tom}`" :style="{ '--pct': `${bi.saude.score}%` }">
                            <span class="saude-nota">{{ bi.saude.score }}</span>
                        </div>
                        <span :class="['text-sm font-medium mt-2', tomClasses[bi.tomSaude.tom]]">{{ bi.tomSaude.frase }}</span>
                        <span class="text-xs text-muted-foreground">Saúde do negócio</span>
                    </div>
                    <ul class="saude-componentes flex-1 w-full">
                        <li v-for="c in bi.saude.componentes" :key="c.nome">
                            <div class="flex justify-between text-sm mb-0.5">
                                <span class="font-medium">{{ c.nome }}</span>
                                <span :class="tomClasses[c.nota >= 80 ? 'bom' : c.nota >= 50 ? 'atencao' : 'critico']">{{ c.nota }}</span>
                            </div>
                            <div class="saude-track">
                                <div
                                    class="saude-bar"
                                    :class="c.nota >= 80 ? 'bg-green-500' : c.nota >= 50 ? 'bg-orange-400' : 'bg-red-500'"
                                    :style="{ width: `${Math.max(c.nota, 2)}%` }"
                                />
                            </div>
                            <span class="text-xs text-muted-foreground">{{ c.detalhe }}</span>
                        </li>
                    </ul>
                </div>
            </CardContent>
        </Card>

        <!-- Meta de faturamento do mês -->
        <Card v-if="bi.metaProgresso" class="mb-4">
            <CardContent>
                <div class="flex items-center gap-2 mb-2">
                    <Flag class="text-primary size-4" />
                    <span class="font-semibold">Meta do mês</span>
                    <span class="ml-auto text-sm text-muted-foreground">
                        {{ formatCentavos(bi.metaProgresso.realizado) }} de {{ formatCentavos(bi.metaProgresso.meta) }}
                        ({{ bi.metaProgresso.pct.toFixed(0) }}%)
                    </span>
                </div>
                <div class="meta-track">
                    <div
                        class="meta-bar"
                        :class="bi.metaProgresso.atingida ? 'bg-green-500' : 'bg-primary'"
                        :style="{ width: `${Math.max(bi.metaProgresso.pct, 2)}%` }"
                    />
                </div>
                <p class="text-xs text-muted-foreground mt-2 mb-0">
                    <template v-if="bi.metaProgresso.atingida">🎉 Meta batida! Cada venda daqui pra frente é crescimento puro.</template>
                    <template v-else>
                        Faltam {{ formatCentavos(bi.metaProgresso.falta) }} —
                        cerca de {{ formatCentavos(bi.metaProgresso.porDiaUtil) }} por dia útil
                        ({{ bi.metaProgresso.diasUteisRestantes }} restantes no mês).
                    </template>
                </p>
            </CardContent>
        </Card>

        <!-- Saúde do negócio: KPIs com leitura de situação -->
        <div v-if="bi.indicadores.length" class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 mb-4">
            <Card v-for="ind in bi.indicadores" :key="ind.label">
                <CardContent>
                    <div class="flex items-start justify-between">
                        <div>
                            <span class="text-sm text-muted-foreground">{{ ind.label }}</span>
                            <div :class="['text-2xl font-semibold mt-1', tomClasses[ind.tom]]">
                                <Skeleton v-if="bi.loading" class="h-8 w-24" />
                                <span v-else>{{ ind.value }}</span>
                            </div>
                            <span class="text-xs text-muted-foreground">{{ ind.detalhe }}</span>
                        </div>
                        <div class="flex items-center justify-center w-11 h-11 rounded-lg shrink-0 bg-primary/10">
                            <component :is="iconePi(ind.icon)" class="text-primary size-5" />
                        </div>
                    </div>
                </CardContent>
            </Card>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 mb-4">
            <NuxtLink v-for="card in vm.cards" :key="card.label" :to="card.to" class="block">
                <Card class="hover:ring-primary/50 transition-shadow">
                    <CardContent>
                        <div class="flex items-start justify-between">
                            <div>
                                <span class="text-sm text-muted-foreground">{{ card.label }}</span>
                                <div class="text-2xl font-semibold mt-1">
                                    <Skeleton v-if="vm.loading" class="h-8 w-24" />
                                    <span v-else>{{ card.value }}</span>
                                </div>
                            </div>
                            <div :class="['flex items-center justify-center w-11 h-11 rounded-lg shrink-0', card.bg]">
                                <component :is="iconePi(card.icon)" :class="[card.color, 'size-5']" />
                            </div>
                        </div>
                    </CardContent>
                </Card>
            </NuxtLink>
        </div>

        <div class="grid grid-cols-1 xl:grid-cols-12 gap-4">
            <div class="xl:col-span-8 flex flex-col gap-4">
                <Card v-if="bi.temDadosReceita">
                    <CardContent>
                        <p class="text-base font-semibold">Quanto você vendeu por dia (últimos 30 dias)</p>
                        <p class="text-sm text-muted-foreground mb-3">Quando a linha sobe, foi um dia bom de vendas; quando desce, foi mais fraco</p>
                        <ChartContainer :config="bi.chartReceitaConfig" class="h-56 w-full">
                            <VisXYContainer :data="bi.chartReceitaData" :margin="{ left: -24 }" :y-domain="[0, undefined]">
                                <VisArea
                                    :x="(_d: any, i: number) => i"
                                    :y="(d: any) => d.receita"
                                    color="var(--color-receita)"
                                    :opacity="0.15"
                                />
                                <VisLine
                                    :x="(_d: any, i: number) => i"
                                    :y="(d: any) => d.receita"
                                    color="var(--color-receita)"
                                    :curve-type="CurveType.MonotoneX"
                                />
                                <VisAxis
                                    type="x"
                                    :x="(_d: any, i: number) => i"
                                    :tick-line="false"
                                    :domain-line="false"
                                    :grid-line="false"
                                    :num-ticks="8"
                                    :tick-format="(_d: number, i: number) => bi.chartReceitaData[i]?.dia ?? ''"
                                />
                                <VisAxis type="y" :num-ticks="3" :tick-line="false" :domain-line="false" />
                                <ChartTooltip />
                                <ChartCrosshair
                                    :template="componentToString(bi.chartReceitaConfig, ChartTooltipContent, { labelKey: 'dia', hideLabel: true })"
                                    color="var(--color-receita)"
                                />
                            </VisXYContainer>
                        </ChartContainer>
                    </CardContent>
                </Card>


                <Card>
                    <CardContent>
                        <p class="text-base font-semibold mb-3">Vendas por status</p>
                        <ChartContainer :config="vm.chartVendasStatusConfig" class="h-64 w-full">
                            <VisXYContainer :data="vm.chartVendasStatusData" :margin="{ left: -24 }" :y-domain="[0, undefined]">
                                <VisGroupedBar
                                    :x="(_d: any, i: number) => i"
                                    :y="(d: any) => d.quantidade"
                                    :color="(d: any) => d.fill"
                                    :rounded-corners="6"
                                />
                                <VisAxis
                                    type="x"
                                    :x="(_d: any, i: number) => i"
                                    :tick-line="false"
                                    :domain-line="false"
                                    :grid-line="false"
                                    :tick-format="(_d: number, i: number) => vm.chartVendasStatusData[i]?.status ?? ''"
                                />
                                <VisAxis type="y" :num-ticks="4" :tick-line="false" :domain-line="false" />
                                <ChartTooltip />
                                <ChartCrosshair
                                    :template="componentToString(vm.chartVendasStatusConfig, ChartTooltipContent, { labelKey: 'status', hideLabel: true })"
                                    color="#0000"
                                />
                            </VisXYContainer>
                        </ChartContainer>
                    </CardContent>
                </Card>
            </div>

            <div class="xl:col-span-4 flex flex-col gap-4">
                <Card>
                    <CardContent>
                        <p class="text-base font-semibold mb-3">Estoque baixo</p>
                        <ul v-if="vm.estoqueBaixo.length" class="flex flex-col gap-4">
                            <li v-for="item in vm.estoqueBaixo" :key="item.produto_id">
                                <div class="flex justify-between mb-1 text-sm">
                                    <span class="font-medium">{{ item.produto?.descricao ?? item.produto_id }}</span>
                                    <span class="text-muted-foreground">{{ item.quantidade }}/{{ item.estoque_minimo }}</span>
                                </div>
                                <div class="estoque-track">
                                    <div
                                        class="estoque-bar"
                                        :class="item.percentual < 50 ? 'bg-red-500' : 'bg-orange-500'"
                                        :style="{ width: `${item.percentual}%` }"
                                    />
                                </div>
                            </li>
                        </ul>
                        <p v-else class="text-muted-foreground text-sm">Nenhum produto abaixo do mínimo.</p>
                    </CardContent>
                </Card>

                <Card>
                    <CardContent>
                        <p class="text-base font-semibold mb-3">Atividade recente</p>
                        <ul v-if="vm.atividadeRecente.length" class="flex flex-col divide-y divide-border overflow-hidden rounded-lg border">
                            <li v-for="(item, idx) in vm.atividadeRecente" :key="idx">
                                <NuxtLink :to="item.to" class="group flex items-center gap-3 px-3 py-2.5 transition-colors hover:bg-muted">
                                    <span :class="['flex size-8 shrink-0 items-center justify-center rounded-full text-white', item.color]">
                                        <component :is="iconePi(item.icon)" class="size-4" />
                                    </span>
                                    <span class="min-w-0 flex-1">
                                        <p class="truncate text-sm font-medium">{{ item.titulo }}</p>
                                        <p class="truncate text-xs text-muted-foreground">{{ item.descricao }}</p>
                                    </span>
                                    <ArrowRight class="size-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
                                </NuxtLink>
                            </li>
                        </ul>
                        <p v-else class="text-muted-foreground text-sm">Sem atividade recente.</p>
                    </CardContent>
                </Card>
            </div>
        </div>
    </div>
    </NuxtLayout>
</template>

<style scoped>
.alerta-lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
}

.alerta-item {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
    padding: 0.5rem 0;
}

.alerta-item + .alerta-item {
    border-top: 1px solid var(--border);
}

.alerta-icone {
    margin-top: 0.2rem;
    color: var(--color-orange-500, #f97316);
}

/* ── Meta do mês ── */
.meta-track {
    height: 10px;
    border-radius: 6px;
    background: var(--border);
    overflow: hidden;
}

.meta-bar {
    height: 100%;
    border-radius: 6px;
    transition: width 0.4s ease;
}

/* ── Estoque baixo ── */
.estoque-track {
    height: 6px;
    border-radius: 4px;
    background: var(--border);
    overflow: hidden;
}

.estoque-bar {
    height: 100%;
    border-radius: 4px;
    transition: width 0.4s ease;
}

/* ── Score de saúde ── */
.saude-anel {
    width: 7rem;
    height: 7rem;
    border-radius: 9999px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: conic-gradient(var(--anel-cor) var(--pct), var(--border) 0);
}

.saude-anel::before {
    content: '';
    position: absolute;
    width: 5.75rem;
    height: 5.75rem;
    border-radius: 9999px;
    background: var(--card, #fff);
}

.saude-anel { position: relative; }

.saude-nota {
    position: relative;
    font-size: 1.75rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
}

.saude-bom { --anel-cor: #22c55e; }
.saude-atencao { --anel-cor: #fb923c; }
.saude-critico { --anel-cor: #ef4444; }

.saude-componentes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
    gap: 0.75rem 1.5rem;
}

.saude-track {
    height: 5px;
    border-radius: 4px;
    background: var(--border);
    overflow: hidden;
    margin-bottom: 0.125rem;
}

.saude-bar {
    height: 100%;
    border-radius: 4px;
}
</style>
