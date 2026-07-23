<script setup lang="ts">
import { ArrowRight, CirclePlus, HelpCircle, Info, LoaderCircle, Package, Phone, Tag, TrendingUp, TriangleAlert, Wallet } from '@lucide/vue'
import { VisAxis, VisGroupedBar, VisXYContainer } from '@unovis/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import {
    ChartContainer,
    ChartCrosshair,
    ChartTooltip,
    ChartTooltipContent,
    componentToString,
} from '@/components/ui/chart'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

const vm = useAnalisesViewModel()
const precos = useAnalisePrecosViewModel()
const { formatCentavos } = useFormat()

onMounted(() => {
    vm.carregar()
    precos.carregar()
})

const tomTexto: Record<string, string> = {
    bom: 'text-green-600 dark:text-green-400',
    atencao: 'text-orange-500',
    critico: 'text-red-500',
}

const statusLabel: Record<string, string> = {
    rascunho: 'Rascunho',
    emitido: 'Emitidos',
    aceito: 'Aceitos',
    convertido: 'Convertidos',
    recusado: 'Recusados',
    expirado: 'Expirados',
    cancelado: 'Cancelados',
}

const statusTom: Record<string, string> = {
    rascunho: 'text-muted-foreground',
    emitido: 'text-sky-600 dark:text-sky-400',
    aceito: 'text-sky-600 dark:text-sky-400',
    convertido: 'text-green-600 dark:text-green-400',
    recusado: 'text-red-500',
    expirado: 'text-orange-500',
    cancelado: 'text-red-500',
}

const importanciaLabel: Record<string, string> = {
    A: 'Muito importante',
    B: 'Importância média',
    C: 'Pouco importante',
}

/** Intensidade da célula na matriz ABC×XYZ — quanto mais importante a linha
 * (A), mais forte o destaque, reforçando visualmente onde prestar atenção.
 * Usa a cor de destaque do tenant (`--primary`, definida pelo whitelabel via
 * useMarca; emerald no padrão) misturada com transparente, para acompanhar a
 * marca e adaptar a claro/escuro sem cores fixas. */
const abcTomStyle: Record<string, { backgroundColor: string }> = {
    A: { backgroundColor: 'color-mix(in oklab, var(--primary) 28%, transparent)' },
    B: { backgroundColor: 'color-mix(in oklab, var(--primary) 15%, transparent)' },
    C: { backgroundColor: 'color-mix(in oklab, var(--primary) 8%, transparent)' },
}
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Análises</h1>
            <p class="text-muted-foreground">Como está o dinheiro, as vendas e o estoque do seu negócio — em linguagem simples.</p>
        </div>

        <div v-if="vm.loading" class="py-16 text-center">
            <LoaderCircle class="mx-auto size-12 animate-spin text-muted-foreground" />
        </div>

        <Tabs v-else default-value="financeiro">
            <TabsList>
                <TabsTrigger value="financeiro"><Wallet class="size-4 mr-2" />Dinheiro e Caixa</TabsTrigger>
                <TabsTrigger value="comercial"><TrendingUp class="size-4 mr-2" />Vendas e Clientes</TabsTrigger>
                <TabsTrigger value="estoque"><Package class="size-4 mr-2" />Estoque e Compras</TabsTrigger>
                <TabsTrigger value="precos"><Tag class="size-4 mr-2" />Preços e Margens</TabsTrigger>
            </TabsList>

            <!-- ── Dinheiro e Caixa ───────────────────────────────────── -->
            <TabsContent value="financeiro">
                <MessageBox severity="info" class="mb-4">
                    {{ vm.explicacaoCiclo }}
                </MessageBox>

                <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 mb-4">
                    <Card v-for="c in vm.cicloCards" :key="c.label">
                        <CardContent>
                            <span class="text-sm text-muted-foreground">{{ c.label }}</span>
                            <div :class="['text-2xl font-semibold mt-1', tomTexto[c.tom]]">{{ c.dias.toFixed(0) }} dias</div>
                            <span class="text-xs text-muted-foreground">{{ c.meta }}</span>
                        </CardContent>
                    </Card>
                </div>

                <div class="grid grid-cols-1 xl:grid-cols-2 gap-4 mb-4">
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">O que deve entrar e sair de dinheiro (próximas 12 semanas)</p>
                            <p class="text-sm text-muted-foreground mb-3">Barra verde = dinheiro entrando · barra vermelha = dinheiro saindo, semana a semana</p>
                            <ChartContainer :config="vm.chartProjecaoConfig" class="h-64 w-full">
                                <VisXYContainer :data="vm.chartProjecaoData" :margin="{ left: -24 }">
                                    <VisGroupedBar
                                        :x="(_d: any, i: number) => i"
                                        :y="[(d: any) => d.receber, (d: any) => d.pagar]"
                                        :color="[vm.chartProjecaoConfig.receber.color, vm.chartProjecaoConfig.pagar.color]"
                                        :rounded-corners="4"
                                    />
                                    <VisAxis
                                        type="x"
                                        :x="(_d: any, i: number) => i"
                                        :tick-line="false"
                                        :domain-line="false"
                                        :grid-line="false"
                                        :num-ticks="6"
                                        :tick-format="(_d: number, i: number) => vm.chartProjecaoData[i]?.semana ?? ''"
                                    />
                                    <VisAxis type="y" :num-ticks="4" :tick-line="false" :domain-line="false" />
                                    <ChartTooltip />
                                    <ChartCrosshair
                                        :template="componentToString(vm.chartProjecaoConfig, ChartTooltipContent, { indicator: 'dashed', labelKey: 'semana' })"
                                        color="#0000"
                                    />
                                </VisXYContainer>
                            </ChartContainer>
                        </CardContent>
                    </Card>
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Quanto os clientes te devem</p>
                            <p class="text-sm text-muted-foreground mb-3">Separado por: ainda no prazo, ou há quanto tempo já venceu</p>
                            <ChartContainer :config="vm.chartAgingConfig" class="h-64 w-full">
                                <VisXYContainer :data="vm.chartAgingData" :margin="{ left: -24 }" :y-domain="[0, undefined]">
                                    <VisGroupedBar
                                        :x="(_d: any, i: number) => i"
                                        :y="(d: any) => d.saldo"
                                        :color="(d: any) => d.fill"
                                        :rounded-corners="6"
                                    />
                                    <VisAxis
                                        type="x"
                                        :x="(_d: any, i: number) => i"
                                        :tick-line="false"
                                        :domain-line="false"
                                        :grid-line="false"
                                        :tick-format="(_d: number, i: number) => vm.chartAgingData[i]?.faixa ?? ''"
                                    />
                                    <VisAxis type="y" :num-ticks="4" :tick-line="false" :domain-line="false" />
                                    <ChartTooltip />
                                    <ChartCrosshair
                                        :template="componentToString(vm.chartAgingConfig, ChartTooltipContent, { labelKey: 'faixa', hideLabel: true })"
                                        color="#0000"
                                    />
                                </VisXYContainer>
                            </ChartContainer>
                        </CardContent>
                    </Card>
                </div>

                <Card>
                    <CardContent>
                        <p class="text-base font-semibold mb-3">Quem mais te deve — comece cobrando estes</p>
                        <AppDataTable
                            :rows="vm.financeiro?.devedores ?? []"
                            row-key="nome"
                            empty-text="Ninguém está te devendo. 🎉"
                            :page-size-options="[]"
                            :columns="[
                                { key: 'nome', label: 'Cliente' },
                                { key: 'saldo', label: 'Quanto deve' },
                                { key: 'atraso', label: 'Atrasado há' },
                                { key: 'acoes', label: '', class: 'w-32' },
                            ]"
                        >
                            <template #cell-saldo="{ row }">{{ formatCentavos(row.saldo_centavos) }}</template>
                            <template #cell-atraso="{ row }">{{ row.dias_atraso }} dia(s)</template>
                            <template #cell-acoes>
                                <NuxtLink to="/financeiro">
                                    <Button variant="outline" size="sm">
                                        <Phone class="size-4" />
                                        Cobrar
                                    </Button>
                                </NuxtLink>
                            </template>
                        </AppDataTable>
                    </CardContent>
                </Card>
            </TabsContent>

            <!-- ── Vendas e Clientes ──────────────────────────────────── -->
            <TabsContent value="comercial">
                <p class="text-sm text-muted-foreground mb-3">
                    Caminho que um orçamento percorre até virar venda (ou não). Passe da esquerda para a direita.
                </p>
                <div class="grid grid-cols-[repeat(auto-fit,minmax(7rem,1fr))] gap-3 mb-4">
                    <Card v-for="etapa in vm.funilOrdenado" :key="etapa.status">
                        <CardContent class="p-4 text-center">
                            <span class="text-xs text-muted-foreground">{{ statusLabel[etapa.status] ?? etapa.status }}</span>
                            <div :class="['text-xl font-semibold', statusTom[etapa.status]]">{{ etapa.quantidade }}</div>
                            <span class="text-xs text-muted-foreground">{{ formatCentavos(etapa.total_centavos) }}</span>
                        </CardContent>
                    </Card>
                </div>
                <MessageBox v-if="vm.conversaoFunil !== null" :severity="vm.conversaoFunil >= 30 ? 'success' : vm.conversaoFunil >= 15 ? 'warn' : 'error'" class="mb-4">
                    Nos últimos 3 meses, de cada 10 orçamentos que você fez, <strong>{{ Math.round(vm.conversaoFunil / 10) }}</strong> viraram venda
                    ({{ vm.conversaoFunil.toFixed(0) }}%). O normal no mercado é entre 2 e 3 a cada 10 (20% a 35%).
                </MessageBox>

                <div class="grid grid-cols-1 xl:grid-cols-2 gap-4 mb-4">
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Orçamentos que estão prestes a vencer</p>
                            <p class="text-sm text-muted-foreground mb-3">Ligue para estes clientes antes que o orçamento vença</p>
                            <AppDataTable
                                :rows="vm.comercial?.expirando ?? []"
                                row-key="cliente"
                                empty-text="Nenhum orçamento vencendo nos próximos 3 dias."
                                :page-size-options="[]"
                                :columns="[
                                    { key: 'cliente', label: 'Cliente' },
                                    { key: 'valor', label: 'Valor' },
                                    { key: 'vence', label: 'Vence em' },
                                    { key: 'acoes', label: '', class: 'w-32' },
                                ]"
                            >
                                <template #cell-valor="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
                                <template #cell-vence="{ row }">
                                    <StatusBadge :value="`${row.vence_em_dias} dia(s)`" :severity="row.vence_em_dias <= 1 ? 'danger' : 'warn'" />
                                </template>
                                <template #cell-acoes>
                                    <NuxtLink to="/orcamentos">
                                        <Button variant="outline" size="sm">
                                            Abrir
                                            <ArrowRight class="size-4" />
                                        </Button>
                                    </NuxtLink>
                                </template>
                            </AppDataTable>
                        </CardContent>
                    </Card>
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Clientes bons que sumiram</p>
                            <p class="text-sm text-muted-foreground mb-3">Compravam bastante, mas faz tempo que não voltam — vale uma ligação</p>
                            <AppDataTable
                                :rows="vm.comercial?.em_risco ?? []"
                                row-key="nome"
                                empty-text="Nenhum cliente importante sumiu."
                                :page-size-options="[]"
                                :columns="[
                                    { key: 'nome', label: 'Cliente' },
                                    { key: 'valor', label: 'Já comprou (12 meses)' },
                                    { key: 'recencia', label: 'Sem comprar há' },
                                    { key: 'contato', label: 'Contato' },
                                ]"
                            >
                                <template #cell-valor="{ row }">{{ formatCentavos(row.valor_12m_centavos) }}</template>
                                <template #cell-recencia="{ row }">{{ row.recencia_dias }} dias</template>
                                <template #cell-contato="{ row }">{{ row.telefone || row.email || '—' }}</template>
                            </AppDataTable>
                        </CardContent>
                    </Card>
                </div>

                <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold mb-3">Como cada vendedor está se saindo (últimos 3 meses)</p>
                            <AppDataTable
                                :rows="vm.comercial?.vendedores ?? []"
                                row-key="vendedor"
                                empty-text="Sem vendas no período."
                                :page-size-options="[]"
                                :columns="[
                                    { key: 'vendedor', label: 'Vendedor' },
                                    { key: 'receita', label: 'Total vendido' },
                                    { key: 'vendas', label: 'Nº de vendas' },
                                    { key: 'ticket', label: 'Valor médio por venda' },
                                    { key: 'conversao', label: 'Fecha quantos orçamentos' },
                                    { key: 'desconto', label: 'Desconto que costuma dar' },
                                ]"
                            >
                                <template #cell-receita="{ row }">{{ formatCentavos(row.receita_centavos) }}</template>
                                <template #cell-ticket="{ row }">{{ formatCentavos(row.ticket_centavos) }}</template>
                                <template #cell-conversao="{ row }">{{ row.conversao_percent === null ? '—' : `${row.conversao_percent.toFixed(0)}%` }}</template>
                                <template #cell-desconto="{ row }">{{ row.desconto_percent === null ? '—' : `${row.desconto_percent.toFixed(1)}%` }}</template>
                            </AppDataTable>
                        </CardContent>
                    </Card>
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Seus clientes, agrupados por importância</p>
                            <p class="text-sm text-muted-foreground mb-3">Baseado em: há quanto tempo compraram, com que frequência, e quanto gastaram</p>
                            <AppDataTable
                                :rows="vm.comercial?.rfm ?? []"
                                row-key="segmento"
                                empty-text="Sem clientes identificados nas vendas — peça o CPF/CNPJ no caixa."
                                :page-size-options="[]"
                                :columns="[
                                    { key: 'segmento', label: 'Grupo' },
                                    { key: 'clientes', label: 'Quantos clientes' },
                                    { key: 'valor', label: 'Quanto gastaram (12 meses)' },
                                ]"
                            >
                                <template #cell-segmento="{ row }">
                                    <StatusBadge :value="row.segmento" :severity="vm.segmentoSeverity[row.segmento] ?? 'secondary'" />
                                </template>
                                <template #cell-valor="{ row }">{{ formatCentavos(row.valor_centavos) }}</template>
                            </AppDataTable>
                            <p class="text-xs text-muted-foreground mt-3">
                                <strong>Campeão</strong>: compra sempre e gasta bem — cuide bem dele.
                                <strong>Em risco</strong>: já gastou bem, mas sumiu — ligue antes que esqueça de você.
                                <strong>Perdido</strong>: não compra há muito tempo.
                            </p>
                        </CardContent>
                    </Card>
                </div>
            </TabsContent>

            <!-- ── Estoque e Compras ──────────────────────────────────── -->
            <TabsContent value="estoque">
                <div class="grid grid-cols-1 xl:grid-cols-2 gap-4 mb-4">
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Quais produtos merecem mais atenção</p>
                            <p class="text-sm text-muted-foreground mb-3">Linhas: o quanto o produto pesa nas suas vendas · Colunas: se ele vende sempre igual ou varia muito</p>
                            <div class="overflow-x-auto">
                                <table class="w-full text-sm">
                                    <thead>
                                        <tr class="text-muted-foreground">
                                            <th class="p-2 text-left" />
                                            <th class="p-2">Vende parecido</th>
                                            <th class="p-2">Vende variável</th>
                                            <th class="p-2">Vende imprevisível</th>
                                            <th class="p-2">Sem vendas</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr v-for="linha in vm.matriz" :key="linha.abc">
                                            <td class="p-2 font-semibold">{{ importanciaLabel[linha.abc] }}</td>
                                            <td v-for="col in linha.colunas" :key="col.xyz" class="p-2 text-center rounded">
                                                <div v-if="col.celula" class="rounded-lg py-2 px-1" :style="abcTomStyle[linha.abc]">
                                                    <div class="font-semibold">{{ col.celula.produtos }} produto(s)</div>
                                                    <div class="text-xs text-muted-foreground">{{ formatCentavos(col.celula.valor_centavos) }}</div>
                                                </div>
                                                <span v-else class="text-muted-foreground">—</span>
                                            </td>
                                        </tr>
                                    </tbody>
                                </table>
                            </div>
                            <p class="text-xs text-muted-foreground mt-3">
                                Se faltar um produto da linha <strong>"Muito importante"</strong>, você perde venda na hora.
                                Se um produto <strong>"Pouco importante"</strong> estiver parado e sobrando, é dinheiro preso — melhor vender mais barato ou devolver.
                            </p>
                        </CardContent>
                    </Card>
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold mb-3">Quais categorias vendem rápido e dão lucro</p>
                            <div class="max-h-64 overflow-y-auto">
                                <AppDataTable
                                    :rows="vm.estoque?.categorias ?? []"
                                    row-key="categoria"
                                    empty-text="Sem dados."
                                    :page-size-options="[]"
                                    :columns="[
                                        { key: 'categoria', label: 'Categoria' },
                                        { key: 'receita', label: 'Total vendido' },
                                        { key: 'margem', label: 'Lucro (margem)' },
                                        { key: 'valor_estoque', label: 'Preso em estoque' },
                                        { key: 'giro', label: 'Vezes que virou dinheiro no ano' },
                                    ]"
                                >
                                    <template #cell-receita="{ row }">{{ formatCentavos(row.receita_centavos) }}</template>
                                    <template #cell-margem="{ row }">{{ row.margem_percent === null ? '—' : `${row.margem_percent.toFixed(0)}%` }}</template>
                                    <template #cell-valor_estoque="{ row }">{{ formatCentavos(row.valor_estoque_centavos) }}</template>
                                    <template #cell-giro="{ row }">{{ row.giro === null ? '—' : `${row.giro.toFixed(1)}×` }}</template>
                                </AppDataTable>
                            </div>
                        </CardContent>
                    </Card>
                </div>

                <div class="grid grid-cols-1 xl:grid-cols-2 gap-4 mb-4">
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Produtos que podem faltar em breve</p>
                            <p class="text-sm text-muted-foreground mb-3">No limite do estoque mínimo — os mais importantes aparecem primeiro</p>
                            <AppDataTable
                                :rows="vm.estoque?.rupturas ?? []"
                                row-key="sku"
                                empty-text="Nenhum produto correndo risco de faltar."
                                :page-size-options="[]"
                                :columns="[
                                    { key: 'produto', label: 'Produto' },
                                    { key: 'importancia', label: 'Importância' },
                                    { key: 'quantidade', label: 'Tem hoje' },
                                    { key: 'cobertura', label: 'Dá para quantos dias' },
                                    { key: 'acoes', label: '', class: 'w-40' },
                                ]"
                            >
                                <template #cell-produto="{ row }">{{ row.sku }} — {{ row.descricao }}</template>
                                <template #cell-importancia="{ row }">
                                    <StatusBadge :value="importanciaLabel[row.classe_abc]" :severity="row.classe_abc === 'A' ? 'danger' : row.classe_abc === 'B' ? 'warn' : 'secondary'" />
                                </template>
                                <template #cell-cobertura="{ row }">{{ row.cobertura_dias === null ? '—' : `${row.cobertura_dias} dia(s)` }}</template>
                                <template #cell-acoes="{ row }">
                                    <NuxtLink :to="vm.linkPedido(row)">
                                        <Button size="sm">
                                            <CirclePlus class="size-4" />
                                            Pedir {{ row.sugestao_compra }} un
                                        </Button>
                                    </NuxtLink>
                                </template>
                            </AppDataTable>
                        </CardContent>
                    </Card>
                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Produtos parados, sem vender</p>
                            <p class="text-sm text-muted-foreground mb-3">Dinheiro preso há mais de 90 dias — pense em promover ou devolver</p>
                            <AppDataTable
                                :rows="vm.estoque?.mortos ?? []"
                                row-key="sku"
                                empty-text="Nenhum produto parado. 🎉"
                                :page-size-options="[]"
                                :columns="[
                                    { key: 'produto', label: 'Produto' },
                                    { key: 'quantidade', label: 'Qtd.' },
                                    { key: 'valor', label: 'Dinheiro preso' },
                                    { key: 'dias', label: 'Sem vender há' },
                                ]"
                            >
                                <template #cell-produto="{ row }">{{ row.sku }} — {{ row.descricao }}</template>
                                <template #cell-valor="{ row }">{{ formatCentavos(row.valor_centavos) }}</template>
                                <template #cell-dias="{ row }">{{ row.dias_sem_venda === null ? 'nunca vendeu' : `${row.dias_sem_venda} dias` }}</template>
                            </AppDataTable>
                        </CardContent>
                    </Card>
                </div>

                <Card>
                    <CardContent>
                        <p class="text-base font-semibold">Pedidos de compra esquecidos</p>
                        <p class="text-sm text-muted-foreground mb-3">Aprovados há mais de 7 dias e ainda não foram enviados ao fornecedor</p>
                        <AppDataTable
                            :rows="vm.estoque?.pedidos_parados ?? []"
                            row-key="fornecedor"
                            empty-text="Nenhum pedido esquecido."
                            :page-size-options="[]"
                            :columns="[
                                { key: 'fornecedor', label: 'Fornecedor' },
                                { key: 'total', label: 'Total' },
                                { key: 'status', label: 'Status' },
                                { key: 'dias_parado', label: 'Parado há' },
                                { key: 'acoes', label: '', class: 'w-32' },
                            ]"
                        >
                            <template #cell-total="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
                            <template #cell-dias_parado="{ row }">{{ row.dias_parado }} dia(s)</template>
                            <template #cell-acoes>
                                <NuxtLink to="/compras">
                                    <Button variant="outline" size="sm">
                                        Abrir
                                        <ArrowRight class="size-4" />
                                    </Button>
                                </NuxtLink>
                            </template>
                        </AppDataTable>
                    </CardContent>
                </Card>
            </TabsContent>

            <!-- ── Preços e Margens ───────────────────────────────────── -->
            <TabsContent value="precos">
                <div v-if="precos.loading" class="py-8 text-center">
                    <LoaderCircle class="mx-auto size-10 animate-spin text-muted-foreground" />
                </div>

                <MessageBox v-else-if="!precos.configurado" severity="info">
                    Configure a margem padrão da loja em <NuxtLink to="/configuracoes" class="underline">Configurações</NuxtLink>
                    para ver a análise de preços.
                </MessageBox>

                <template v-else>
                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-4">
                        <Card>
                            <CardContent>
                                <span class="text-sm text-muted-foreground">Produtos rendendo menos que o desejado</span>
                                <div class="text-2xl font-semibold mt-1">{{ precos.abaixoDoAlvo.length }}</div>
                                <span class="text-xs text-muted-foreground">margem atual abaixo da meta</span>
                            </CardContent>
                        </Card>
                        <Card>
                            <CardContent>
                                <span class="text-sm text-muted-foreground">Se ajustar os preços sugeridos</span>
                                <div class="text-2xl font-semibold mt-1 text-green-600 dark:text-green-400">
                                    +{{ formatCentavos(precos.ganhoPotencialTotal) }}
                                </div>
                                <span class="text-xs text-muted-foreground">vendendo o estoque que você já tem</span>
                            </CardContent>
                        </Card>
                        <Card>
                            <CardContent>
                                <span class="text-sm text-muted-foreground">Parados — merecem desconto para girar</span>
                                <div class="text-2xl font-semibold mt-1 text-orange-500">{{ precos.encalhados.length }}</div>
                                <span class="text-xs text-muted-foreground">a sugestão já reduz a margem destes</span>
                            </CardContent>
                        </Card>
                    </div>

                    <Card v-if="precos.cobertura" class="mb-4">
                        <CardContent>
                            <p class="text-base font-semibold">Cobertura dos custos fixos</p>
                            <p class="text-sm text-muted-foreground mb-3">
                                Cada preço já embute a mesma fração para pagar os custos fixos (aluguel, salário,
                                DAS…) — sem penalizar item caro. Aqui você confere se, no volume esperado, isso
                                realmente fecha a conta do mês.
                            </p>
                            <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                                <div>
                                    <span class="text-sm text-muted-foreground">Custos fixos por mês</span>
                                    <div class="text-2xl font-semibold mt-1">
                                        {{ formatCentavos(precos.cobertura.fixosCentavos) }}
                                    </div>
                                </div>
                                <div v-if="precos.cobertura.equilibrio">
                                    <span class="text-sm text-muted-foreground">Ponto de equilíbrio</span>
                                    <div class="text-2xl font-semibold mt-1">
                                        {{ precos.cobertura.equilibrio.unidades }} vendas/mês
                                    </div>
                                    <span class="text-xs text-muted-foreground">
                                        ≈ {{ formatCentavos(precos.cobertura.equilibrio.receitaCentavos) }} para pagar os fixos
                                    </span>
                                </div>
                                <div>
                                    <span class="text-sm text-muted-foreground">Situação</span>
                                    <div
                                        v-if="precos.cobertura.cobre === true"
                                        class="text-2xl font-semibold mt-1 text-green-600 dark:text-green-400"
                                    >
                                        Cobre ✓
                                    </div>
                                    <div
                                        v-else-if="precos.cobertura.cobre === false"
                                        class="text-2xl font-semibold mt-1 text-orange-500"
                                    >
                                        Faltam {{ formatCentavos(precos.cobertura.gapMensalCentavos) }}/mês
                                    </div>
                                    <div v-else class="text-sm mt-1 text-muted-foreground">
                                        Informe as vendas/mês esperadas em Configurações.
                                    </div>
                                    <span v-if="precos.cobertura.cobre === false" class="text-xs text-muted-foreground">
                                        suba a margem nas categorias de menos concorrência, ou venda mais
                                    </span>
                                </div>
                            </div>
                            <p v-if="precos.cobertura.markupCoberturaPct != null" class="text-xs text-muted-foreground mt-3">
                                Cada preço já reserva
                                <strong>{{ Number(precos.cobertura.markupCoberturaPct.toFixed(1)) }}%</strong>
                                (custos fixos ÷ faturamento esperado) para os custos fixos. A margem que você
                                define por categoria é o LUCRO por cima disso — item de nicho aguenta mais,
                                commodity menos (cada caso é um caso).
                            </p>
                        </CardContent>
                    </Card>

                    <Card>
                        <CardContent>
                            <p class="text-base font-semibold">Preço praticado × preço sugerido</p>
                            <p class="text-sm text-muted-foreground mb-3">
                                Ordenado pela maior diferença. Clique em "Por quê?" para ver o passo a passo do cálculo.
                            </p>
                            <AppDataTable
                                :rows="precos.linhas"
                                row-key="produto.sku"
                                empty-text="Nenhum produto ativo com custo cadastrado."
                                search-placeholder="Buscar por SKU, descrição ou categoria"
                                :search-fields="['produto.sku', 'produto.descricao', 'produto.categoria']"
                                :page-size="15"
                                :page-size-options="[15, 30, 50, 100]"
                                :columns="[
                                    { key: 'produto.sku', label: 'Produto', sortable: true },
                                    { key: 'produto.preco_custo', label: 'Custo', sortable: true },
                                    { key: 'produto.preco_venda', label: 'Preço hoje', sortable: true },
                                    { key: 'margemAtualPct', label: 'Margem hoje', sortable: true },
                                    { key: 'deltaCentavos', label: 'Sugerido', sortable: true },
                                    { key: 'acoes', label: '', class: 'w-28' },
                                ]"
                            >
                                <template #cell-produto.sku="{ row }">
                                    <span class="font-medium">{{ row.produto.sku }}</span>
                                    <span class="text-muted-foreground"> — {{ row.produto.descricao }}</span>
                                    <StatusBadge v-if="row.encalhado" value="parado" severity="warn" class="ml-2" />
                                </template>
                                <template #cell-produto.preco_custo="{ row }">{{ formatCentavos(row.produto.preco_custo) }}</template>
                                <template #cell-produto.preco_venda="{ row }">{{ formatCentavos(row.produto.preco_venda) }}</template>
                                <template #cell-margemAtualPct="{ row }">
                                    <span :class="row.margemAtualPct != null && row.margemAtualPct < row.sugestao.margemPct - 2 ? 'text-red-500' : 'text-green-600 dark:text-green-400'">
                                        {{ row.margemAtualPct == null ? '—' : `${row.margemAtualPct.toFixed(0)}%` }}
                                    </span>
                                    <span class="text-xs text-muted-foreground"> / meta {{ row.sugestao.margemPct }}%</span>
                                </template>
                                <template #cell-deltaCentavos="{ row }">
                                    <TriangleAlert
                                        v-if="row.custoMedioAcimaCadastro"
                                        class="inline size-3.5 mr-1 text-orange-500 align-[-2px]"
                                        :title="`Custo médio do estoque (${formatCentavos(row.custoMedioAcimaCadastro.custoMedioCentavos)}) muito acima do cadastro (${formatCentavos(row.custoMedioAcimaCadastro.custoCadastroCentavos)}) — provável erro na entrada. A sugestão usa o médio; confira antes de aplicar.`"
                                    />
                                    <strong>{{ formatCentavos(row.sugestao.precoCentavos) }}</strong>
                                    <span v-if="row.deltaCentavos !== 0" :class="['text-xs ml-1', row.deltaCentavos > 0 ? 'text-green-600 dark:text-green-400' : 'text-orange-500']">
                                        ({{ row.deltaCentavos > 0 ? '+' : '' }}{{ formatCentavos(row.deltaCentavos) }})
                                    </span>
                                </template>
                                <template #cell-acoes="{ row }">
                                    <Button variant="ghost" size="sm" @click="precos.detalhe = row">
                                        <HelpCircle class="size-4" />
                                        Por quê?
                                    </Button>
                                </template>
                            </AppDataTable>
                        </CardContent>
                    </Card>

                    <Dialog :open="precos.detalhe != null" @update:open="(v) => { if (!v) precos.detalhe = null }">
                        <DialogContent class="sm:max-w-xl">
                            <DialogHeader>
                                <DialogTitle>{{ precos.detalhe ? `${precos.detalhe.produto.sku} — por que ${formatCentavos(precos.detalhe.sugestao.precoCentavos)}?` : '' }}</DialogTitle>
                            </DialogHeader>
                            <ExplicacaoPreco
                                v-if="precos.detalhe"
                                :sugestao="precos.detalhe.sugestao"
                                :custo-centavos="precos.detalhe.custoBaseCentavos"
                            />
                            <!-- Descompasso GRANDE: provável erro de entrada — alerta forte. -->
                            <MessageBox
                                v-if="precos.detalhe && precos.detalhe.custoMedioAcimaCadastro"
                                severity="warn"
                                class="mt-2"
                            >
                                O custo médio de estoque deste item
                                (<strong>{{ formatCentavos(precos.detalhe.custoMedioAcimaCadastro.custoMedioCentavos) }}</strong>)
                                está muito acima do custo de cadastro
                                (<strong>{{ formatCentavos(precos.detalhe.custoMedioAcimaCadastro.custoCadastroCentavos) }}</strong>)
                                — provável erro em alguma entrada de estoque. Como a sugestão usa o maior custo, ela fica
                                inflada. <strong>Confira as entradas deste produto</strong> antes de aplicar o preço.
                            </MessageBox>
                            <!-- Diferença normal de compra: só uma nota informativa. -->
                            <p
                                v-else-if="precos.detalhe && precos.detalhe.custoBaseCentavos > precos.detalhe.produto.preco_custo"
                                class="text-xs text-muted-foreground mt-2 flex items-start gap-1"
                            >
                                <Info class="size-3.5 mt-0.5 shrink-0" />
                                O custo usado é o custo médio real do
                                estoque ({{ formatCentavos(precos.detalhe.custoBaseCentavos) }}), maior que o
                                do cadastro ({{ formatCentavos(precos.detalhe.produto.preco_custo) }}) — a
                                sugestão nunca finge que o estoque custou mais barato.
                            </p>
                            <div class="flex justify-end mt-4">
                                <NuxtLink to="/catalogo">
                                    <Button variant="outline" size="sm">
                                        Ajustar no catálogo
                                        <ArrowRight class="size-4" />
                                    </Button>
                                </NuxtLink>
                            </div>
                        </DialogContent>
                    </Dialog>
                </template>
            </TabsContent>
        </Tabs>
    </div>
</template>
