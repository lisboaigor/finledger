<script setup lang="ts">
import { Check, Lock, LockOpen, LoaderCircle, Pencil, Plus, RefreshCw, Search, Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Textarea } from '@/components/ui/textarea'

const vm = useClientesViewModel()

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Clientes</h1>
            <p class="text-muted-foreground">Cadastro de clientes (CRM).</p>
        </div>

        <div class="mb-4 flex justify-between">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filtro" placeholder="Buscar por nome, CPF/CNPJ ou e-mail" />
            </InputGroup>
            <Button @click="vm.abrirNovo">
                <Plus class="size-4" />
                Novo Cliente
            </Button>
        </div>

        <AppDataTable
            :rows="vm.filtrados"
            :loading="vm.loading"
            row-key="cliente_id"
            empty-text="Nenhum cliente encontrado."
            :columns="[
                { key: 'nome', label: 'Nome', sortable: true },
                { key: 'cpf_cnpj', label: 'CPF/CNPJ' },
                { key: 'telefone', label: 'Telefone' },
                { key: 'email', label: 'E-mail' },
                { key: 'situacao', label: 'Situação' },
                { key: 'acoes', label: 'Ações', class: 'w-32' },
            ]"
        >
            <template #cell-telefone="{ row }">{{ row.telefone || '—' }}</template>
            <template #cell-email="{ row }">{{ row.email || '—' }}</template>
            <template #cell-situacao="{ row }">
                <StatusBadge
                    :value="!row.ativo ? 'Inativo' : row.bloqueado ? 'Bloqueado' : 'Ativo'"
                    :severity="!row.ativo ? 'secondary' : row.bloqueado ? 'danger' : 'success'"
                />
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button variant="ghost" size="icon-sm" @click="vm.abrirEdicao(row)">
                        <Pencil class="size-4" />
                    </Button>
                    <Button
                        v-if="vm.isAdmin && !row.bloqueado"
                        variant="ghost"
                        size="icon-sm"
                        class="text-destructive"
                        @click="vm.abrirBloqueio(row)"
                    >
                        <Lock class="size-4" />
                    </Button>
                    <Button
                        v-if="vm.isAdmin && row.bloqueado"
                        variant="ghost"
                        size="icon-sm"
                        class="text-emerald-600"
                        @click="vm.desbloquear(row)"
                    >
                        <LockOpen class="size-4" />
                    </Button>
                    <Button
                        v-if="vm.isAdmin"
                        variant="ghost"
                        size="icon-sm"
                        :class="row.ativo ? 'text-destructive' : 'text-emerald-600'"
                        :title="row.ativo ? 'Excluir' : 'Reativar'"
                        @click="row.ativo ? vm.abrirExclusao(row) : vm.alternarAtivo(row)"
                    >
                        <Trash2 v-if="row.ativo" class="size-4" />
                        <RefreshCw v-else class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <Dialog v-model:open="vm.dialogVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>{{ vm.editando ? 'Editar Cliente' : 'Novo Cliente' }}</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4">
                    <Field>
                        <FieldLabel>Nome</FieldLabel>
                        <Input v-model="vm.form.nome" />
                    </Field>
                    <Field>
                        <FieldLabel>CPF / CNPJ</FieldLabel>
                        <Input v-model="vm.form.cpf_cnpj" :disabled="!!vm.editando" />
                    </Field>
                    <div class="grid grid-cols-2 gap-3">
                        <Field>
                            <FieldLabel>Telefone</FieldLabel>
                            <Input v-model="vm.form.telefone" />
                        </Field>
                        <Field>
                            <FieldLabel>E-mail</FieldLabel>
                            <Input v-model="vm.form.email" />
                        </Field>
                    </div>
                    <Field>
                        <FieldLabel>UF (destinatário)</FieldLabel>
                        <Input v-model="vm.form.uf" maxlength="2" placeholder="Ex.: SP" />
                        <p class="text-xs text-muted-foreground">
                            Usada na nota fiscal para o CFOP (operação dentro ou fora do estado).
                        </p>
                    </Field>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.dialogVisible = false">Cancelar</Button>
                    <Button :disabled="vm.salvando" @click="vm.salvar">
                        <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <Dialog v-model:open="vm.bloqueioVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Bloquear Cliente</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel>Motivo do bloqueio</FieldLabel>
                    <Textarea v-model="vm.motivo" rows="3" />
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.bloqueioVisible = false">Cancelar</Button>
                    <Button variant="destructive" :disabled="!vm.motivo" @click="vm.bloquear">
                        <Lock class="size-4" />
                        Bloquear
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
