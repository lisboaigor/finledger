import type { MonthlyRevenue, RevenueOverview, TenantRevenue } from '~/models/backoffice'
import { getRevenueOverview } from '~/models/backoffice'

export interface MonthBar {
    /** first day of the month, ISO date */
    month: string
    label: string
    totalCents: number
    salesCount: number
}

export interface DayPoint {
    day: string
    label: string
    totalCents: number
    salesCount: number
}

export interface TenantRow extends TenantRevenue {
    /** last-12-months revenue, oldest first — feeds the sparkline */
    spark: number[]
    /** growth of the last 30 days vs the 30 days before; null when prev = 0 */
    growth: number | null
}

export interface RankedTenant {
    tenant_id: string
    nome: string
    slug: string
    cents: number
    share: number
}

export interface PlanSlice {
    plano: string
    tenantCount: number
    cents: number
    share: number
}

const MONTH_LABELS = ['jan', 'fev', 'mar', 'abr', 'mai', 'jun', 'jul', 'ago', 'set', 'out', 'nov', 'dez']

function monthIso(d: Date): string {
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-01`
}

function lastMonths(count: number): { iso: string; date: Date }[] {
    const now = new Date()
    return Array.from({ length: count }, (_, i) => {
        const d = new Date(now.getFullYear(), now.getMonth() - (count - 1 - i), 1)
        return { iso: monthIso(d), date: d }
    })
}

/** Fills the last `count` months with zeros so charts always show a full,
 * gap-free timeline even when the API only returns months that had sales. */
function buildMonthSeries(monthly: MonthlyRevenue[], count: number): MonthBar[] {
    const byMonth = new Map(monthly.map((m) => [m.month, m]))
    return lastMonths(count).map(({ iso, date }, i) => {
        const row = byMonth.get(iso)
        return {
            month: iso,
            label: `${MONTH_LABELS[date.getMonth()]}${date.getMonth() === 0 || i === 0 ? `/${String(date.getFullYear()).slice(2)}` : ''}`,
            totalCents: row?.total_cents ?? 0,
            salesCount: row?.sales_count ?? 0,
        }
    })
}

function buildDaySeries(daily: { day: string; total_cents: number; sales_count: number }[], count: number): DayPoint[] {
    const byDay = new Map(daily.map((d) => [d.day, d]))
    const points: DayPoint[] = []
    const now = new Date()
    for (let i = count - 1; i >= 0; i--) {
        const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - i)
        const iso = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
        const row = byDay.get(iso)
        points.push({
            day: iso,
            label: `${String(d.getDate()).padStart(2, '0')}/${String(d.getMonth() + 1).padStart(2, '0')}`,
            totalCents: row?.total_cents ?? 0,
            salesCount: row?.sales_count ?? 0,
        })
    }
    return points
}

/** ViewModel of the backoffice overview page: platform KPIs with growth vs
 * the previous window, daily/monthly revenue series, tenant ranking, plan
 * breakdown and per-tenant rows with sparklines. */
export function useBackofficeDashboardViewModel() {
    const { apiFetch, apiErrorMessage } = useBackofficeApi()
    const { hasPermission, token } = useBackofficeAuth()
    const { notifyError } = useNotify()

    const canRead = computed(() => hasPermission('tenants:read'))

    function verificarAcesso() {
        if (!token.value) navigateTo('/login')
    }

    const loading = ref(false)
    const overview = ref<RevenueOverview | null>(null)

    async function carregar() {
        if (!canRead.value) return
        loading.value = true
        try {
            overview.value = await getRevenueOverview(apiFetch)
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    // ── Series ──────────────────────────────────────────────────────────────
    const months = computed(() => buildMonthSeries(overview.value?.monthly ?? [], 12))
    const days = computed(() => buildDaySeries(overview.value?.daily ?? [], 30))

    // ── Tenant rows (sparkline + growth) ────────────────────────────────────
    const tenants = computed<TenantRow[]>(() => {
        const scaffold = lastMonths(12)
        const sparkByTenant = new Map<string, Map<string, number>>()
        for (const m of overview.value?.monthly_by_tenant ?? []) {
            const inner = sparkByTenant.get(m.tenant_id) ?? new Map<string, number>()
            inner.set(m.month, m.total_cents)
            sparkByTenant.set(m.tenant_id, inner)
        }
        return (overview.value?.tenants ?? []).map((t) => ({
            ...t,
            spark: scaffold.map(({ iso }) => sparkByTenant.get(t.tenant_id)?.get(iso) ?? 0),
            growth:
                t.prev_30d_cents > 0
                    ? (t.last_30d_cents - t.prev_30d_cents) / t.prev_30d_cents
                    : null,
        }))
    })

    // ── KPIs ────────────────────────────────────────────────────────────────
    const stats = computed(() => overview.value?.stats ?? null)
    const totalCents = computed(() => tenants.value.reduce((s, t) => s + t.total_cents, 0))
    const last30dCents = computed(() => tenants.value.reduce((s, t) => s + t.last_30d_cents, 0))
    const last30dCount = computed(() => tenants.value.reduce((s, t) => s + t.last_30d_count, 0))
    const prev30dCents = computed(() => tenants.value.reduce((s, t) => s + t.prev_30d_cents, 0))
    const growth30d = computed(() =>
        prev30dCents.value > 0
            ? (last30dCents.value - prev30dCents.value) / prev30dCents.value
            : null,
    )
    const avgTicket30dCents = computed(() =>
        last30dCount.value > 0 ? Math.round(last30dCents.value / last30dCount.value) : 0,
    )
    const activeTenants = computed(() => tenants.value.filter((t) => t.status === 'ativo').length)

    // ── Ranking (top 5 by last-30d revenue) ─────────────────────────────────
    const topTenants = computed<RankedTenant[]>(() => {
        const ranked = [...tenants.value]
            .filter((t) => t.last_30d_cents > 0)
            .sort((a, b) => b.last_30d_cents - a.last_30d_cents)
            .slice(0, 5)
        const max = ranked[0]?.last_30d_cents ?? 0
        return ranked.map((t) => ({
            tenant_id: t.tenant_id,
            nome: t.nome,
            slug: t.slug,
            cents: t.last_30d_cents,
            share: max > 0 ? t.last_30d_cents / max : 0,
        }))
    })

    // ── Plan breakdown (30d revenue per plan) ───────────────────────────────
    const planBreakdown = computed<PlanSlice[]>(() => {
        const order = ['basico', 'profissional', 'enterprise']
        const slices = new Map<string, PlanSlice>()
        for (const t of tenants.value) {
            const s = slices.get(t.plano) ?? { plano: t.plano, tenantCount: 0, cents: 0, share: 0 }
            s.tenantCount += 1
            s.cents += t.last_30d_cents
            slices.set(t.plano, s)
        }
        const total = last30dCents.value
        return [...slices.values()]
            .map((s) => ({ ...s, share: total > 0 ? s.cents / total : 0 }))
            .sort((a, b) => order.indexOf(a.plano) - order.indexOf(b.plano))
    })

    const filters = ref({ global: { value: null as string | null, matchMode: 'contains' } })

    return reactive({
        canRead,
        verificarAcesso,
        loading,
        carregar,
        tenants,
        months,
        days,
        stats,
        totalCents,
        last30dCents,
        last30dCount,
        growth30d,
        avgTicket30dCents,
        activeTenants,
        topTenants,
        planBreakdown,
        filters,
    })
}
