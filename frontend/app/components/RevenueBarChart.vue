<template>
    <div v-if="empty" class="relative">
        <ChartContainer :config="chartConfig" class="h-60 w-full opacity-30">
            <VisXYContainer :data="chartData" :margin="{ left: -24 }" :y-domain="[0, undefined]">
                <VisGroupedBar :x="(_d: Row, i: number) => i" :y="(d: Row) => d.receita" color="var(--color-receita)" :rounded-corners="4" />
            </VisXYContainer>
        </ChartContainer>
        <div class="absolute inset-0 flex items-center justify-center text-sm text-muted-foreground">
            Sem vendas confirmadas no período.
        </div>
    </div>
    <ChartContainer v-else :config="chartConfig" class="h-60 w-full">
        <VisXYContainer :data="chartData" :margin="{ left: -24 }" :y-domain="[0, undefined]">
            <VisGroupedBar
                :x="(_d: Row, i: number) => i"
                :y="(d: Row) => d.receita"
                color="var(--color-receita)"
                :rounded-corners="4"
            />
            <VisAxis
                type="x"
                :x="(_d: Row, i: number) => i"
                :tick-line="false"
                :domain-line="false"
                :grid-line="false"
                :tick-format="(_d: number, i: number) => chartData[i]?.label ?? ''"
            />
            <VisAxis type="y" :num-ticks="3" :tick-line="false" :domain-line="false" />
            <ChartTooltip />
            <ChartCrosshair
                :template="componentToString(chartConfig, ChartTooltipContent, { labelKey: 'label', hideLabel: true })"
                color="#0000"
            />
        </VisXYContainer>
    </ChartContainer>
</template>

<script setup lang="ts">
import type { ChartConfig } from '@/components/ui/chart'
import type { MonthBar } from '~/composables/viewmodels/useBackofficeDashboardViewModel'
import { VisAxis, VisGroupedBar, VisXYContainer } from '@unovis/vue'
import {
    ChartContainer,
    ChartCrosshair,
    ChartTooltip,
    ChartTooltipContent,
    componentToString,
} from '@/components/ui/chart'

interface Row { label: string, receita: number }

const props = defineProps<{ months: MonthBar[] }>()

const empty = computed(() => props.months.every((m) => m.totalCents === 0))

const chartData = computed<Row[]>(() =>
    props.months.map((m) => ({ label: m.label, receita: m.totalCents / 100 })),
)

const chartConfig = {
    receita: { label: 'Faturamento', color: 'var(--chart-1)' },
} satisfies ChartConfig
</script>
