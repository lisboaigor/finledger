<script setup lang="ts" generic="T extends Record<string, any>">
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { ArrowDown, ArrowUp, ArrowUpDown, LoaderCircle, Search } from '@lucide/vue'

export interface DataTableColumn {
  /** Chave lida em `row[key]` para a renderização padrão e para ordenação.
   * Colunas puramente de ação (sem dado) podem usar uma chave livre. */
  key: string
  label: string
  sortable?: boolean
  class?: string
  align?: 'left' | 'right' | 'center'
}

const props = withDefaults(
  defineProps<{
    rows: T[]
    columns: DataTableColumn[]
    loading?: boolean
    rowKey: string
    emptyText?: string
    /** Habilita a busca embutida (filtra por `searchFields`). Deixe de fora
     * quando a página já filtra os dados antes de passá-los para a tabela. */
    searchFields?: string[]
    searchPlaceholder?: string
    pageSize?: number
    pageSizeOptions?: number[]
    striped?: boolean
    initialSortKey?: string
    initialSortDesc?: boolean
  }>(),
  {
    loading: false,
    emptyText: 'Nenhum registro encontrado.',
    searchFields: undefined,
    searchPlaceholder: 'Buscar...',
    pageSize: 10,
    pageSizeOptions: () => [10, 25, 50, 100],
    striped: true,
    initialSortKey: undefined,
    initialSortDesc: false,
  },
)

function getPath(row: T, path: string) {
  return path.split('.').reduce<any>((acc, k) => acc?.[k], row)
}

const search = ref('')
const sortKey = ref(props.initialSortKey)
const sortDesc = ref(props.initialSortDesc)
const page = ref(1)
const perPage = ref(props.pageSize)

function toggleSort(col: DataTableColumn) {
  if (!col.sortable) return
  if (sortKey.value !== col.key) {
    sortKey.value = col.key
    sortDesc.value = false
  } else if (!sortDesc.value) {
    sortDesc.value = true
  } else {
    sortKey.value = undefined
  }
  page.value = 1
}

const filtered = computed(() => {
  if (!props.searchFields?.length || !search.value.trim()) return props.rows
  const term = search.value.trim().toLowerCase()
  return props.rows.filter((row) =>
    props.searchFields!.some((field) => String(getPath(row, field) ?? '').toLowerCase().includes(term)),
  )
})

const sorted = computed(() => {
  if (!sortKey.value) return filtered.value
  const key = sortKey.value
  const dir = sortDesc.value ? -1 : 1
  return [...filtered.value].sort((a, b) => {
    const av = getPath(a, key)
    const bv = getPath(b, key)
    if (av == null && bv == null) return 0
    if (av == null) return -1 * dir
    if (bv == null) return 1 * dir
    if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * dir
    return String(av).localeCompare(String(bv), 'pt-BR') * dir
  })
})

const totalPages = computed(() => Math.max(1, Math.ceil(sorted.value.length / perPage.value)))

watch([filtered, perPage], () => {
  if (page.value > totalPages.value) page.value = totalPages.value
})

const paginated = computed(() => {
  const start = (page.value - 1) * perPage.value
  return sorted.value.slice(start, start + perPage.value)
})

const rangeLabel = computed(() => {
  if (!sorted.value.length) return '0 de 0'
  const start = (page.value - 1) * perPage.value + 1
  const end = Math.min(sorted.value.length, page.value * perPage.value)
  return `${start}–${end} de ${sorted.value.length}`
})
</script>

<template>
  <div class="flex min-w-0 flex-col gap-3">
    <div v-if="searchFields?.length" class="relative max-w-sm">
      <Search class="absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
      <input
        v-model="search"
        type="text"
        :placeholder="searchPlaceholder"
        class="border-input bg-background h-8 w-full rounded-lg border pl-8 pr-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-3"
        @input="page = 1"
      >
    </div>

    <slot name="header" />

    <div class="min-w-0 rounded-lg border">
      <Table>
        <TableHeader>
          <TableRow :class="{ 'hover:bg-transparent': true }">
            <TableHead
              v-for="col in columns"
              :key="col.key"
              :class="[
                col.class,
                col.align === 'right' && 'text-right',
                col.align === 'center' && 'text-center',
                col.sortable && 'cursor-pointer select-none',
              ]"
              @click="toggleSort(col)"
            >
              <span class="inline-flex items-center gap-1">
                {{ col.label }}
                <template v-if="col.sortable">
                  <ArrowUp v-if="sortKey === col.key && !sortDesc" class="size-3.5" />
                  <ArrowDown v-else-if="sortKey === col.key && sortDesc" class="size-3.5" />
                  <ArrowUpDown v-else class="size-3.5 text-muted-foreground/50" />
                </template>
              </span>
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-if="loading">
            <TableCell :colspan="columns.length" class="py-10 text-center text-muted-foreground">
              <LoaderCircle class="mx-auto size-5 animate-spin" />
            </TableCell>
          </TableRow>
          <TableRow v-else-if="!paginated.length">
            <TableCell :colspan="columns.length" class="py-10 text-center text-muted-foreground">
              <slot name="empty">{{ emptyText }}</slot>
            </TableCell>
          </TableRow>
          <TableRow
            v-for="row in paginated"
            v-else
            :key="row[rowKey]"
            :class="striped && 'odd:bg-muted/30'"
          >
            <TableCell
              v-for="col in columns"
              :key="col.key"
              :class="[col.class, col.align === 'right' && 'text-right', col.align === 'center' && 'text-center']"
            >
              <slot :name="`cell-${col.key}`" :row="row">{{ getPath(row, col.key) }}</slot>
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </div>

    <div v-if="sorted.length" class="relative flex flex-wrap items-center justify-between gap-2 text-sm text-muted-foreground ">
      <span>{{ rangeLabel }}</span>
      <div class="flex items-center gap-3">
        <select
          v-if="pageSizeOptions.length > 1"
          v-model.number="perPage"
          class="border-input bg-background h-7 rounded-lg border px-1.5 text-xs outline-none"
        >
          <option v-for="n in pageSizeOptions" :key="n" :value="n">{{ n }} / página</option>
        </select>
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="disabled:opacity-40 hover:text-foreground"
            :disabled="page <= 1"
            @click="page--"
          >
            ‹ Anterior
          </button>
          <span class="px-1">{{ page }} / {{ totalPages }}</span>
          <button
            type="button"
            class="disabled:opacity-40 hover:text-foreground"
            :disabled="page >= totalPages"
            @click="page++"
          >
            Próxima ›
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
