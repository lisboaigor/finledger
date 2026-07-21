<script setup lang="ts">
import { Ban, Check, LoaderCircle, Pencil, Plus, Search } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'

const vm = useFornecedoresViewModel()

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Fornecedores</h1>
            <p class="text-muted-foreground">Cadastro de fornecedores.</p>
        </div>

        <div class="mb-4 flex justify-between items-center">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filtro" placeholder="Buscar por razão social ou CNPJ" />
            </InputGroup>
            <Button @click="vm.abrirNovo">
                <Plus class="size-4" />
                Novo Fornecedor
            </Button>
        </div>

        <AppDataTable
            :rows="vm.filtrados"
            :loading="vm.loading"
            row-key="fornecedor_id"
            empty-text="Nenhum fornecedor encontrado."
            :columns="[
                { key: 'razao_social', label: 'Razão Social', sortable: true },
                { key: 'cnpj', label: 'CNPJ' },
                { key: 'situacao', label: 'Situação' },
                { key: 'acoes', label: 'Ações', class: 'w-32' },
            ]"
        >
            <template #cell-situacao="{ row }">
                <StatusBadge :value="row.ativo ? 'Ativo' : 'Inativo'" :severity="row.ativo ? 'success' : 'danger'" />
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button variant="ghost" size="icon-sm" @click="vm.abrirEdicao(row)">
                        <Pencil class="size-4" />
                    </Button>
                    <Button
                        v-if="vm.isAdmin"
                        variant="ghost"
                        size="icon-sm"
                        :class="row.ativo ? 'text-destructive' : 'text-emerald-600'"
                        @click="vm.alternarAtivo(row)"
                    >
                        <Ban v-if="row.ativo" class="size-4" />
                        <Check v-else class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <Dialog v-model:open="vm.dialogVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>{{ vm.editando ? 'Editar Fornecedor' : 'Novo Fornecedor' }}</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4 pt-2">
                    <Field>
                        <FieldLabel>Razão Social</FieldLabel>
                        <Input v-model="vm.form.razao_social" />
                    </Field>
                    <Field>
                        <FieldLabel>CNPJ</FieldLabel>
                        <Input v-model="vm.form.cnpj" :disabled="!!vm.editando" />
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
                        <FieldLabel>Prazo de pagamento (dias)</FieldLabel>
                        <InputQuantity v-model="vm.form.prazo_pagamento_dias" :min="0" :max="365" />
                    </Field>
                    <Field v-if="vm.editando">
                        <FieldLabel>Frete típico da compra (%)</FieldLabel>
                        <InputPercent v-model="vm.freteTipicoPct" :min="0" :max="99" :max-fraction-digits="2" />
                        <p class="text-xs text-muted-foreground">
                            Quanto este fornecedor costuma cobrar de frete sobre o valor da mercadoria —
                            pré-preenche o frete ao registrar entradas de estoque dele.
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
    </div>
</template>
