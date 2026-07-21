<script setup lang="ts">
import { FileEdit, History, LoaderCircle, ShoppingCart } from '@lucide/vue'
import type { Orcamento } from '~/models/orcamentos'
import type { Venda } from '~/models/vendas'
import { Button } from '@/components/ui/button'
import { Dialog, DialogHeader, DialogScrollContent, DialogTitle } from '@/components/ui/dialog'

const props = defineProps<{
    carregando: boolean
    vendas: Venda[]
    orcamentos: Orcamento[]
    clienteLabel: (clienteId: string | null, clienteAvulso?: string | null) => string
    formatCentavos: (centavos: number) => string
}>()

const emit = defineEmits<{
    'retomar-venda': [id: string]
    'retomar-orcamento': [id: string]
}>()

const visible = defineModel<boolean>('visible', { required: true })

const vazio = computed(
    () => !props.carregando && props.vendas.length === 0 && props.orcamentos.length === 0,
)
</script>

<template>
    <Dialog v-model:open="visible">
        <DialogScrollContent class="sm:max-w-2xl">
            <DialogHeader>
                <DialogTitle>Recuperar documento em aberto</DialogTitle>
            </DialogHeader>

            <div v-if="carregando" class="py-10 text-center text-muted-foreground">
                <LoaderCircle class="mx-auto size-6 animate-spin" />
            </div>

            <div v-else-if="vazio" class="py-10 text-center text-sm text-muted-foreground">
                <History class="mx-auto mb-2 size-8 opacity-50" />
                Nada para recuperar — sem vendas em andamento nem orçamentos em aberto.
            </div>

            <div v-else class="flex flex-col gap-5">
                <!-- Vendas em andamento -->
                <section v-if="vendas.length" class="flex flex-col gap-2">
                    <h3 class="flex items-center gap-2 text-sm font-semibold">
                        <ShoppingCart class="size-4" /> Vendas em andamento
                    </h3>
                    <ul class="flex flex-col gap-1.5">
                        <li
                            v-for="v in vendas"
                            :key="v.venda_id"
                            class="flex items-center gap-3 rounded-lg border px-3 py-2"
                        >
                            <div class="min-w-0 flex-1">
                                <div class="truncate text-sm font-medium">
                                    {{ clienteLabel(v.cliente_id) }}
                                </div>
                                <div class="text-xs text-muted-foreground">
                                    #{{ v.venda_id.slice(0, 8) }}
                                </div>
                            </div>
                            <span class="shrink-0 font-mono text-sm">{{ formatCentavos(v.total_centavos) }}</span>
                            <Button size="sm" class="shrink-0" @click="emit('retomar-venda', v.venda_id)">
                                Retomar
                            </Button>
                        </li>
                    </ul>
                </section>

                <!-- Orçamentos em aberto -->
                <section v-if="orcamentos.length" class="flex flex-col gap-2">
                    <h3 class="flex items-center gap-2 text-sm font-semibold">
                        <FileEdit class="size-4" /> Orçamentos em aberto
                    </h3>
                    <ul class="flex flex-col gap-1.5">
                        <li
                            v-for="o in orcamentos"
                            :key="o.orcamento_id"
                            class="flex items-center gap-3 rounded-lg border px-3 py-2"
                        >
                            <div class="min-w-0 flex-1">
                                <div class="truncate text-sm font-medium">
                                    {{ clienteLabel(o.cliente_id, o.cliente_avulso) }}
                                </div>
                                <div class="text-xs text-muted-foreground">
                                    #{{ o.orcamento_id.slice(0, 8) }} · {{ o.status }}
                                </div>
                            </div>
                            <span class="shrink-0 font-mono text-sm">{{ formatCentavos(o.total_centavos) }}</span>
                            <Button size="sm" variant="outline" class="shrink-0" @click="emit('retomar-orcamento', o.orcamento_id)">
                                Retomar
                            </Button>
                        </li>
                    </ul>
                </section>
            </div>
        </DialogScrollContent>
    </Dialog>
</template>
