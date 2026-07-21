<template>
    <ChartContainer :config="chartConfig" class="aspect-auto h-6 w-24">
        <VisXYContainer :data="chartData" :y-domain="[0, undefined]">
            <VisGroupedBar
                :x="(_d: Row, i: number) => i"
                :y="(d: Row) => d.valor"
                :color="(d: Row) => (d.valor === 0 ? 'var(--border)' : 'var(--color-valor)')"
                :rounded-corners="1"
                bar-padding="0.15"
            />
        </VisXYContainer>
    </ChartContainer>
</template>

<script setup lang="ts">
import type { ChartConfig } from '@/components/ui/chart'
import { VisGroupedBar, VisXYContainer } from '@unovis/vue'
import { ChartContainer } from '@/components/ui/chart'

interface Row { valor: number }

const props = defineProps<{ values: number[] }>()

const chartData = computed<Row[]>(() => props.values.map((v) => ({ valor: v })))

const chartConfig = {
    valor: { label: 'Valor', color: 'var(--chart-1)' },
} satisfies ChartConfig
</script>
