<script setup lang="ts">
definePageMeta({ layout: 'backoffice' })

import { Check, KeyRound, LoaderCircle, Pencil, Plus, RefreshCw, Search, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'

const vm = useAdminsViewModel()

onMounted(() => {
    vm.verificarAcesso()
    vm.carregar()
})
</script>

<template>
    <div>
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Admins de Suporte</h1>
            <p class="text-muted-foreground">Gerencie quem tem acesso ao backoffice.</p>
        </div>

        <div class="mb-4 flex items-center justify-between gap-2 border-b pb-3">
            <InputGroup class="max-w-sm">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filters.global.value" placeholder="Buscar admin..." />
            </InputGroup>
            <Button @click="vm.novoVisible = true">
                <Plus class="size-4" />
                Novo Admin
            </Button>
        </div>

        <Card>
            <CardContent>
                <AppDataTable
                    :rows="vm.admins"
                    :loading="vm.loading"
                    row-key="user_id"
                    empty-text="Nenhum admin cadastrado."
                    search-placeholder="Buscar admin..."
                    :search-fields="['username']"
                    :columns="[
                        { key: 'username', label: 'Usuário' },
                        { key: 'role', label: 'Papel' },
                        { key: 'permissions', label: 'Permissões' },
                        { key: 'ativo', label: 'Status', class: 'w-28' },
                        { key: 'acoes', label: 'Ações', class: 'w-40' },
                    ]"
                >
                    <template #cell-role="{ row }">
                        <StatusBadge
                            :value="row.role === 'superadmin' ? 'Superadmin' : 'Admin'"
                            :severity="row.role === 'superadmin' ? 'danger' : 'info'"
                        />
                    </template>
                    <template #cell-permissions="{ row }">
                        <div v-if="row.role === 'superadmin'" class="text-sm text-muted-foreground">
                            Todas
                        </div>
                        <div v-else class="flex flex-wrap gap-1">
                            <StatusBadge v-for="p in row.permissions" :key="p" :value="p" severity="secondary" class="text-[0.7rem]" />
                            <span v-if="!row.permissions.length" class="text-sm text-muted-foreground">—</span>
                        </div>
                    </template>
                    <template #cell-ativo="{ row }">
                        <StatusBadge :value="row.ativo ? 'Ativo' : 'Inativo'" :severity="row.ativo ? 'success' : 'secondary'" />
                    </template>
                    <template #cell-acoes="{ row }">
                        <div class="flex gap-1">
                            <Button
                                v-if="row.role !== 'superadmin'"
                                variant="ghost"
                                size="icon-sm"
                                title="Editar permissões"
                                @click="vm.abrirPermissoes(row)"
                            >
                                <Pencil class="size-4" />
                            </Button>
                            <Button
                                v-if="row.role !== 'superadmin'"
                                variant="ghost"
                                size="icon-sm"
                                title="Redefinir senha"
                                @click="vm.openPasswordReset(row)"
                            >
                                <KeyRound class="size-4" />
                            </Button>
                            <Button
                                v-if="row.role !== 'superadmin' && row.ativo"
                                variant="ghost"
                                size="icon-sm"
                                class="text-destructive"
                                title="Desativar"
                                @click="vm.desativar(row.user_id)"
                            >
                                <X class="size-4" />
                            </Button>
                            <Button
                                v-if="row.role !== 'superadmin' && !row.ativo"
                                variant="ghost"
                                size="icon-sm"
                                class="text-emerald-600"
                                title="Reativar"
                                @click="vm.reativar(row.user_id)"
                            >
                                <RefreshCw class="size-4" />
                            </Button>
                        </div>
                    </template>
                </AppDataTable>
            </CardContent>
        </Card>

        <!-- Novo admin -->
        <Dialog v-model:open="vm.novoVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Novo Admin</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-3">
                    <Field>
                        <FieldLabel>Usuário</FieldLabel>
                        <Input v-model="vm.novoForm.username" />
                    </Field>
                    <Field>
                        <FieldLabel>Senha</FieldLabel>
                        <PasswordInput v-model="vm.novoForm.senha" />
                    </Field>
                    <div class="flex flex-col gap-1">
                        <label class="text-sm font-medium">Permissões</label>
                        <div class="flex flex-col gap-2">
                            <div v-for="p in vm.todasPermissoes" :key="p.value" class="flex items-center gap-2">
                                <Checkbox
                                    :id="`perm-${p.value}`"
                                    :model-value="vm.novoForm.permissions.includes(p.value)"
                                    @update:model-value="(checked) => {
                                        if (checked) vm.novoForm.permissions.push(p.value)
                                        else vm.novoForm.permissions = vm.novoForm.permissions.filter((v: string) => v !== p.value)
                                    }"
                                />
                                <label :for="`perm-${p.value}`" class="text-sm">{{ p.label }}</label>
                            </div>
                        </div>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.novoVisible = false">Cancelar</Button>
                    <Button @click="vm.criarAdmin">
                        <LoaderCircle v-if="vm.saving" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Criar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Redefinir senha -->
        <Dialog v-model:open="vm.passwordVisible">
            <DialogContent class="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>Redefinir Senha</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-3">
                    <p class="text-sm text-muted-foreground">
                        Nova senha para <strong>{{ vm.passwordForm.username }}</strong> (mínimo de 8 caracteres).
                    </p>
                    <PasswordInput v-model="vm.passwordForm.password" autocomplete="new-password" class="w-full" />
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.passwordVisible = false">Cancelar</Button>
                    <Button :disabled="vm.passwordForm.password.length < 8" @click="vm.savePassword">
                        <LoaderCircle v-if="vm.saving" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Editar permissões -->
        <Dialog v-model:open="vm.permissoesVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Permissões</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-2">
                    <div v-for="p in vm.todasPermissoes" :key="p.value" class="flex items-center gap-2">
                        <Checkbox
                            :id="`edit-perm-${p.value}`"
                            :model-value="vm.permissoesForm.permissions.includes(p.value)"
                            @update:model-value="(checked) => {
                                if (checked) vm.permissoesForm.permissions.push(p.value)
                                else vm.permissoesForm.permissions = vm.permissoesForm.permissions.filter((v: string) => v !== p.value)
                            }"
                        />
                        <label :for="`edit-perm-${p.value}`" class="text-sm">{{ p.label }}</label>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.permissoesVisible = false">Cancelar</Button>
                    <Button @click="vm.salvarPermissoes">
                        <LoaderCircle v-if="vm.saving" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
