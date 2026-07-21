<template>
    <div>
        <div class="mb-6 flex items-end justify-between flex-wrap gap-2">
            <div>
                <h1 class="text-2xl font-semibold">Visão Geral</h1>
                <p class="text-muted-foreground">Faturamento consolidado da plataforma (vendas confirmadas).</p>
            </div>
            <Button variant="ghost" size="sm" :disabled="vm.loading" @click="vm.carregar">
                <RefreshCw class="size-4" :class="vm.loading && 'animate-spin'" />
                Atualizar
            </Button>
        </div>

        <MessageBox v-if="!vm.canRead" severity="warn">
            Você não tem a permissão <code>tenants:read</code>. Peça acesso a um superadmin.
        </MessageBox>

        <template v-else>
            <!-- KPIs -->
            <div class="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-4 mb-4">
                <Card>
                    <CardContent>
                        <div class="kpi-label">Receita — 30 dias</div>
                        <div class="kpi-value">{{ formatCentavos(vm.last30dCents) }}</div>
                        <div v-if="vm.growth30d !== null" class="kpi-delta" :class="deltaClass(vm.growth30d)">
                            <component :is="deltaIcon(vm.growth30d)" class="size-3" />
                            {{ formatPercent(vm.growth30d) }} vs 30d anteriores
                        </div>
                        <div v-else class="kpi-delta kpi-delta-muted">sem base de comparação</div>
                    </CardContent>
                </Card>
                <Card>
                    <CardContent>
                        <div class="kpi-label">Vendas — 30 dias</div>
                        <div class="kpi-value">{{ formatNumber(vm.last30dCount) }}</div>
                        <div class="kpi-delta kpi-delta-muted">
                            ticket médio {{ formatCentavos(vm.avgTicket30dCents) }}
                        </div>
                    </CardContent>
                </Card>
                <Card>
                    <CardContent>
                        <div class="kpi-label">Receita — hoje</div>
                        <div class="kpi-value">{{ formatCentavos(vm.stats?.today_cents ?? 0) }}</div>
                        <div class="kpi-delta kpi-delta-muted">
                            {{ formatNumber(vm.stats?.today_count ?? 0) }} venda{{ (vm.stats?.today_count ?? 0) === 1 ? '' : 's' }}
                        </div>
                    </CardContent>
                </Card>
                <Card>
                    <CardContent>
                        <div class="kpi-label">Receita — total</div>
                        <div class="kpi-value">{{ formatCentavos(vm.totalCents) }}</div>
                        <div class="kpi-delta kpi-delta-muted">desde o início</div>
                    </CardContent>
                </Card>
                <Card>
                    <CardContent>
                        <div class="kpi-label">Tenants ativos</div>
                        <div class="kpi-value">{{ vm.activeTenants }}<span class="kpi-of">/ {{ vm.tenants.length }}</span></div>
                        <div class="kpi-delta kpi-delta-muted">
                            {{ vm.tenants.length - vm.activeTenants }} suspenso{{ vm.tenants.length - vm.activeTenants === 1 ? '' : 's' }}
                        </div>
                    </CardContent>
                </Card>
                <Card>
                    <CardContent>
                        <div class="kpi-label">Usuários ativos</div>
                        <div class="kpi-value">{{ formatNumber(vm.stats?.active_users ?? 0) }}</div>
                        <div class="kpi-delta kpi-delta-muted">
                            {{ formatNumber(vm.stats?.total_clients ?? 0) }} clientes ·
                            {{ formatNumber(vm.stats?.total_products ?? 0) }} produtos
                        </div>
                    </CardContent>
                </Card>
            </div>

            <!-- Daily revenue + top tenants -->
            <div class="grid grid-cols-1 xl:grid-cols-3 gap-4 mb-4">
                <Card class="xl:col-span-2">
                    <CardHeader>
                        <CardTitle>Receita diária — últimos 30 dias</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <RevenueAreaChart :days="vm.days" />
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader>
                        <CardTitle>Top tenants — 30 dias</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div v-if="!vm.topTenants.length" class="rank-empty">
                            Sem vendas confirmadas no período.
                        </div>
                        <ol v-else class="rank-list">
                            <li v-for="(t, i) in vm.topTenants" :key="t.tenant_id" class="rank-item">
                                <div class="rank-line">
                                    <span class="rank-pos">{{ i + 1 }}</span>
                                    <span class="rank-name" :title="t.nome">{{ t.nome }}</span>
                                    <span class="rank-value">{{ formatCentavos(t.cents) }}</span>
                                </div>
                                <div class="rank-track">
                                    <div class="rank-bar" :style="{ width: `${Math.max(t.share * 100, 2)}%` }" />
                                </div>
                            </li>
                        </ol>
                    </CardContent>
                </Card>
            </div>

            <!-- Monthly revenue + plan breakdown -->
            <div class="grid grid-cols-1 xl:grid-cols-3 gap-4 mb-4">
                <Card class="xl:col-span-2">
                    <CardHeader>
                        <CardTitle>Receita mensal — últimos 12 meses</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <RevenueBarChart :months="vm.months" />
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader>
                        <CardTitle>Receita 30d por plano</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div v-if="!vm.planBreakdown.length" class="rank-empty">Nenhum tenant cadastrado.</div>
                        <div v-else class="plan-list">
                            <div v-for="p in vm.planBreakdown" :key="p.plano" class="plan-item">
                                <div class="rank-line">
                                    <StatusBadge :value="p.plano" severity="info" />
                                    <span class="plan-count">
                                        {{ p.tenantCount }} tenant{{ p.tenantCount === 1 ? '' : 's' }}
                                    </span>
                                    <span class="rank-value">{{ formatCentavos(p.cents) }}</span>
                                </div>
                                <div class="rank-track">
                                    <div class="rank-bar" :style="{ width: `${Math.max(p.share * 100, 2)}%` }" />
                                </div>
                                <div class="plan-share">{{ formatPercent(p.share, false) }} da receita 30d</div>
                            </div>
                        </div>
                    </CardContent>
                </Card>
            </div>

            <!-- Per-tenant table -->
            <Card>
                <CardHeader>
                    <CardTitle>Faturamento por tenant</CardTitle>
                </CardHeader>
                <CardContent>
                    <AppDataTable
                        :rows="vm.tenants"
                        :loading="vm.loading"
                        row-key="tenant_id"
                        empty-text="Nenhum tenant cadastrado."
                        search-placeholder="Buscar tenant..."
                        :search-fields="['nome', 'slug']"
                        initial-sort-key="last_30d_cents"
                        initial-sort-desc
                        :columns="[
                            { key: 'nome', label: 'Tenant', sortable: true },
                            { key: 'plano', label: 'Plano', sortable: true, class: 'w-32' },
                            { key: 'status', label: 'Status', sortable: true, class: 'w-28' },
                            { key: 'spark', label: '12 meses', class: 'w-32' },
                            { key: 'last_30d_cents', label: 'Receita 30d', sortable: true, class: 'w-40' },
                            { key: 'growth', label: 'Δ 30d', sortable: true, class: 'w-32' },
                            { key: 'total_cents', label: 'Receita total', sortable: true, class: 'w-40' },
                            { key: 'sales_count', label: 'Vendas', sortable: true, class: 'w-28' },
                            { key: 'avg_ticket_cents', label: 'Ticket médio', sortable: true, class: 'w-36' },
                        ]"
                    >
                        <template #cell-nome="{ row }">
                            <div>{{ row.nome }}</div>
                            <div class="text-xs text-muted-foreground">{{ row.slug }}</div>
                        </template>
                        <template #cell-plano="{ row }">
                            <StatusBadge :value="row.plano" severity="info" />
                        </template>
                        <template #cell-status="{ row }">
                            <StatusBadge :value="row.status" :severity="row.status === 'ativo' ? 'success' : 'danger'" />
                        </template>
                        <template #cell-spark="{ row }">
                            <Sparkline :values="row.spark" />
                        </template>
                        <template #cell-last_30d_cents="{ row }">{{ formatCentavos(row.last_30d_cents) }}</template>
                        <template #cell-growth="{ row }">
                            <span v-if="row.growth !== null" :class="deltaClass(row.growth)" class="delta-cell">
                                <component :is="deltaIcon(row.growth)" class="size-3" />
                                {{ formatPercent(row.growth) }}
                            </span>
                            <span v-else-if="row.last_30d_cents > 0" class="delta-cell delta-new">novo</span>
                            <span v-else class="text-muted-foreground">—</span>
                        </template>
                        <template #cell-total_cents="{ row }">{{ formatCentavos(row.total_cents) }}</template>
                        <template #cell-sales_count="{ row }">{{ formatNumber(row.sales_count) }}</template>
                        <template #cell-avg_ticket_cents="{ row }">{{ formatCentavos(row.avg_ticket_cents) }}</template>
                    </AppDataTable>
                </CardContent>
            </Card>
        </template>
    </div>
</template>

<script setup lang="ts">
import { ArrowDownRight, ArrowUpRight, RefreshCw } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

definePageMeta({ layout: 'backoffice' })

const vm = useBackofficeDashboardViewModel()
const { formatCentavos, formatNumber } = useFormat()

const pctFmt = new Intl.NumberFormat('pt-BR', { maximumFractionDigits: 1 })

function formatPercent(ratio: number, signed = true): string {
    const pct = pctFmt.format(Math.abs(ratio) * 100)
    if (!signed) return `${pct}%`
    return `${ratio >= 0 ? '+' : '−'}${pct}%`
}

function deltaClass(ratio: number): string {
    return ratio >= 0 ? 'delta-up' : 'delta-down'
}

function deltaIcon(ratio: number) {
    return ratio >= 0 ? ArrowUpRight : ArrowDownRight
}

onMounted(() => {
    vm.verificarAcesso()
    vm.carregar()
})
</script>

<style scoped>
.kpi-label {
    font-size: 0.8125rem;
    color: var(--muted-foreground);
    margin-bottom: 0.25rem;
}

.kpi-value {
    font-size: 1.5rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    line-height: 1.2;
}

.kpi-of {
    font-size: 0.9375rem;
    color: var(--muted-foreground);
    font-weight: 500;
    margin-left: 0.25rem;
}

.kpi-delta {
    margin-top: 0.375rem;
    font-size: 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.25rem;
}

.kpi-delta-muted {
    color: var(--muted-foreground);
}

.delta-up {
    color: #16a34a;
}

.delta-down {
    color: #dc2626;
}

.delta-cell {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8125rem;
    font-variant-numeric: tabular-nums;
}

.delta-new {
    color: var(--primary);
    font-weight: 500;
}

/* ── Ranking / plan breakdown ── */
.rank-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
}

.rank-item,
.plan-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}

.rank-line {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
}

.rank-pos {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 9999px;
    background: var(--muted);
    color: var(--muted-foreground);
    font-size: 0.6875rem;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

.rank-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.rank-value {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 0.8125rem;
}

.rank-track {
    height: 6px;
    border-radius: 4px;
    background: var(--muted);
    overflow: hidden;
}

.rank-bar {
    height: 100%;
    border-radius: 4px;
    background: #0d9271;
}

.rank-empty {
    color: var(--muted-foreground);
    font-size: 0.875rem;
    padding: 1rem 0;
    text-align: center;
}

.plan-list {
    display: flex;
    flex-direction: column;
    gap: 1.125rem;
}

.plan-count {
    flex: 1;
    color: var(--muted-foreground);
    font-size: 0.8125rem;
}

.plan-share {
    font-size: 0.75rem;
    color: var(--muted-foreground);
}
</style>
