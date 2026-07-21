<script setup lang="ts">
import type { Component } from 'vue'
import {
  BookOpen,
  Boxes,
  ChartLine,
  Cog,
  FileCheck2,
  FileText,
  Home,
  LayoutGrid,
  Monitor,
  ShoppingCart,
  Tag,
  Truck,
  Users,
  Wallet,
} from '@lucide/vue'

export interface MenuEntry {
  label: string
  icon?: Component
  to?: string
}

export interface MenuGroup {
  label: string
  items: MenuEntry[]
}

const props = defineProps<{
  /** Grupos do menu; por padrão, o menu do app do tenant. */
  groups?: MenuGroup[]
}>()

const tenantGroups: MenuGroup[] = [
  {
    label: 'Principal',
    items: [
      { label: 'Dashboard', icon: Home, to: '/' },
      { label: 'Análises', icon: ChartLine, to: '/analises' },
      { label: 'Terminal PDV', icon: Monitor, to: '/terminal' },
    ],
  },
  {
    label: 'Comercial',
    items: [
      { label: 'Vendas', icon: ShoppingCart, to: '/vendas' },
      { label: 'Orçamentos', icon: FileText, to: '/orcamentos' },
      { label: 'Clientes', icon: Users, to: '/clientes' },
    ],
  },
  {
    label: 'Operações',
    items: [
      { label: 'Catálogo', icon: Tag, to: '/catalogo' },
      { label: 'Estoque', icon: Boxes, to: '/estoque' },
      { label: 'Fornecedores', icon: Truck, to: '/fornecedores' },
      { label: 'Compras', icon: LayoutGrid, to: '/compras' },
    ],
  },
  {
    label: 'Financeiro & Fiscal',
    items: [
      { label: 'Financeiro', icon: Wallet, to: '/financeiro' },
      { label: 'Fiscal', icon: FileCheck2, to: '/fiscal' },
    ],
  },
  {
    label: 'Administração',
    items: [
      { label: 'Usuários', icon: Users, to: '/usuarios' },
      { label: 'Configurações', icon: Cog, to: '/configuracoes' },
    ],
  },
  {
    label: 'Ajuda',
    items: [{ label: 'Manual do sistema', icon: BookOpen, to: '/manual' }],
  },
]

const groups = computed(() => props.groups ?? tenantGroups)
</script>

<template>
  <nav class="flex flex-col gap-5 px-3 py-4">
    <div v-for="group in groups" :key="group.label" class="flex flex-col gap-1">
      <span class="px-2.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground/70">
        {{ group.label }}
      </span>
      <AppMenuItem v-for="item in group.items" :key="item.to" :item="item" />
    </div>
  </nav>
</template>
