<template>
    <div v-if="empty" class="relative">
        <ChartContainer :config="chartConfig" class="h-60 w-full opacity-30">
            <VisXYContainer :data="chartData" :margin="{ left: -24 }" :y-domain="[0, undefined]">
                <VisArea :x="(_d: Row, i: number) => i" :y="(d: Row) => d.receita" color="var(--color-receita)" :opacity="0.15" />
                <VisLine :x="(_d: Row, i: number) => i" :y="(d: Row) => d.receita" color="var(--color-receita)" />
            </VisXYContainer>
        </ChartContainer>
        <div class="absolute inset-0 flex items-center justify-center text-sm text-muted-foreground">
            Sem vendas confirmadas no período.
        </div>
    </div>
    <ChartContainer v-else :config="chartConfig" class="h-60 w-full">
        <VisXYContainer :data="chartData" :margin="{ left: -24 }" :y-domain="[0, undefined]">
            <VisArea
                :x="(_d: Row, i: number) => i"
                :y="(d: Row) => d.receita"
                color="var(--color-receita)"
                :opacity="0.15"
            />
            <VisLine
                :x="(_d: Row, i: number) => i"
                :y="(d: Row) => d.receita"
                color="var(--color-receita)"
                :curve-type="CurveType.MonotoneX"
            />
            <VisAxis
                type="x"
                :x="(_d: Row, i: number) => i"
                :tick-line="false"
                :domain-line="false"
                :grid-line="false"
                :num-ticks="8"
                :tick-format="(_d: number, i: number) => chartData[i]?.label ?? ''"
            />
            <VisAxis type="y" :num-ticks="3" :tick-line="false" :domain-line="false" />
            <ChartTooltip />
            <ChartCrosshair
                :template="componentToString(chartConfig, ChartTooltipContent, { labelKey: 'label', hideLabel: true })"
                color="var(--color-receita)"
            />
        </VisXYContainer>
    </ChartContainer>
</template>

<script setup lang="ts">
import type { ChartConfig } from '@/components/ui/chart'
import type { DayPoint } from '~/composables/viewmodels/useBackofficeDashboardViewModel'
import { CurveType } from '@unovis/ts'
import { VisArea, VisAxis, VisLine, VisXYContainer } from '@unovis/vue'
import {
    ChartContainer,
    ChartCrosshair,
    ChartTooltip,
    ChartTooltipContent,
    componentToString,
} from '@/components/ui/chart'

interface Row { label: string, receita: number }

const props = defineProps<{ days: DayPoint[] }>()

const empty = computed(() => props.days.every((d) => d.totalCents === 0))

const chartData = computed<Row[]>(() =>
    props.days.map((d) => ({ label: d.label, receita: d.totalCents / 100 })),
)

const chartConfig = {
    receita: { label: 'Faturamento', color: 'var(--chart-1)' },
} satisfies ChartConfig
</script>
