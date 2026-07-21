<script setup lang="ts">
import { Eye, LoaderCircle, Pencil, Plus, Search, Trash2, Undo2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogHeader, DialogScrollContent, DialogTitle } from '@/components/ui/dialog'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { listarLixeiraOrcamentos, restaurarOrcamento } from '~/models/orcamentos'

const vm = useOrcamentosViewModel()
const { formatCentavos } = useFormat()
const { statusSeverity } = useOrcamentoStatus()
const { isAdmin } = useAuth()
const { apiFetch } = useApi()

const lixeira = useLixeira({
    listar: async () => (await listarLixeiraOrcamentos(apiFetch)).orcamentos,
    restaurar: (id) => restaurarOrcamento(apiFetch, id),
    idDe: (o) => o.orcamento_id,
    aposRestaurar: () => vm.carregar(),
})

// Linhas com o nome do cliente resolvido para a busca global textual.
const lixeiraRows = computed(() =>
    lixeira.itens.map((o) => ({ ...o, cliente_nome: vm.nomeCliente(o) })),
)
const dataCurta = (iso: string) => new Date(iso).toLocaleDateString('pt-BR')

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Orçamentos</h1>
            <p class="text-muted-foreground">Propostas comerciais.</p>
        </div>

        <div class="mb-4 flex justify-between">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filtro" placeholder="Buscar por cliente ou status" />
            </InputGroup>
            <div class="flex shrink-0 gap-2">
                <Button v-if="isAdmin" variant="outline" title="Orçamentos arquivados pela limpeza automática" @click="lixeira.abrir">
                    <Trash2 class="size-4" />
                    Lixeira
                </Button>
                <Button @click="vm.novoVisible = true">
                    <Plus class="size-4" />
                    Novo Orçamento
                </Button>
            </div>
        </div>

        <AppDataTable
            :rows="vm.orcamentosFiltrados"
            :loading="vm.loading"
            row-key="orcamento_id"
            empty-text="Nenhum orçamento."
            :columns="[
                { key: 'cliente', label: 'Cliente' },
                { key: 'desconto', label: 'Desconto' },
                { key: 'total', label: 'Total' },
                { key: 'validade', label: 'Validade' },
                { key: 'status', label: 'Status' },
                { key: 'acoes', label: 'Ações', class: 'w-32' },
            ]"
        >
            <template #cell-cliente="{ row }">{{ vm.nomeCliente(row) }}</template>
            <template #cell-desconto="{ row }">{{ formatCentavos(row.desconto_centavos) }}</template>
            <template #cell-total="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
            <template #cell-validade="{ row }">{{ row.validade_dias }} dias</template>
            <template #cell-status="{ row }">
                <StatusBadge :value="row.status" :severity="statusSeverity(row.status)" />
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button variant="ghost" size="icon-sm" @click="vm.abrirDetalhe(row.orcamento_id)">
                        <Eye class="size-4" />
                    </Button>
                    <Button
                        v-if="row.status === 'Rascunho'"
                        variant="ghost"
                        size="icon-sm"
                        title="Editar"
                        @click="vm.abrirDetalhe(row.orcamento_id)"
                    >
                        <Pencil class="size-4" />
                    </Button>
                    <Button
                        v-if="vm.podeExcluir(row)"
                        variant="ghost"
                        size="icon-sm"
                        class="text-destructive"
                        title="Excluir"
                        @click="vm.abrirExclusao(row)"
                    >
                        <Trash2 class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <!-- Lixeira: orçamentos arquivados pela rotina de limpeza (nada é excluído) -->
        <Dialog v-model:open="lixeira.visible">
            <DialogScrollContent class="sm:max-w-3xl">
                <DialogHeader>
                    <DialogTitle>Lixeira de orçamentos</DialogTitle>
                </DialogHeader>
                <p class="mb-3 text-sm text-muted-foreground">
                    Orçamentos que não viraram venda (rascunhos antigos, recusados, expirados ou
                    cancelados), arquivados automaticamente após o prazo definido em Configurações.
                    Nada foi excluído — restaure para voltar à listagem.
                </p>
                <AppDataTable
                    :rows="lixeiraRows"
                    :loading="lixeira.loading"
                    row-key="orcamento_id"
                    empty-text="Lixeira vazia."
                    search-placeholder="Buscar por cliente ou status"
                    :search-fields="['cliente_nome', 'status']"
                    initial-sort-key="arquivado_em"
                    initial-sort-desc
                    :columns="[
                        { key: 'cliente_nome', label: 'Cliente', sortable: true },
                        { key: 'total_centavos', label: 'Total', sortable: true },
                        { key: 'status', label: 'Status', sortable: true },
                        { key: 'criado_em', label: 'Criado em', sortable: true },
                        { key: 'arquivado_em', label: 'Arquivado em', sortable: true },
                        { key: 'acoes', label: '', class: 'w-32' },
                    ]"
                >
                    <template #cell-total_centavos="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
                    <template #cell-status="{ row }">
                        <StatusBadge :value="row.status" :severity="statusSeverity(row.status)" />
                    </template>
                    <template #cell-criado_em="{ row }">{{ dataCurta(row.criado_em) }}</template>
                    <template #cell-arquivado_em="{ row }">{{ dataCurta(row.arquivado_em) }}</template>
                    <template #cell-acoes="{ row }">
                        <Button
                            variant="outline"
                            size="sm"
                            :disabled="lixeira.restaurando === row.orcamento_id"
                            @click="lixeira.restaurar(row)"
                        >
                            <LoaderCircle v-if="lixeira.restaurando === row.orcamento_id" class="size-4 animate-spin" />
                            <Undo2 v-else class="size-4" />
                            Restaurar
                        </Button>
                    </template>
                </AppDataTable>
            </DialogScrollContent>
        </Dialog>

        <OrcamentoNovoDialog
            v-model:visible="vm.novoVisible"
            :opcoes-cliente="vm.opcoesCliente"
            @criar="vm.criar"
        />

        <OrcamentoDetalheDialog
            v-model:visible="vm.detalheVisible"
            :vm="vm"
        />

        <OrcamentoRecusarDialog
            v-model:visible="vm.recusarVisible"
            @recusar="vm.onRecusar"
        />

        <OrcamentoExcluirDialog
            v-model:visible="vm.excluirVisible"
            @excluir="vm.onExcluir"
        />
    </div>
</template>
