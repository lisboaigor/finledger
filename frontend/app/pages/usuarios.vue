<script setup lang="ts">
import { Check, KeyRound, LoaderCircle, Pencil, Plus, RefreshCw, Search, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { ROLE_LABELS, type RoleName } from '~/composables/useAuth'

const vm = useUsuariosViewModel()

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Usuários</h1>
            <p class="text-muted-foreground">Gerencie quem tem acesso ao sistema desta empresa.</p>
        </div>

        <div class="mb-4 flex items-center justify-between">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filters.global.value" placeholder="Buscar usuário..." />
            </InputGroup>
            <div class="flex gap-2">
                <Button variant="outline" @click="vm.senhaVisible = true">
                    <KeyRound class="size-4" />
                    Alterar minha senha
                </Button>
                <Button v-if="vm.isAdmin" @click="vm.abrirNovo">
                    <Plus class="size-4" />
                    Novo Usuário
                </Button>
            </div>
        </div>

        <AppDataTable
            :rows="vm.usuarios"
            :loading="vm.loading"
            row-key="usuario_id"
            empty-text="Nenhum usuário cadastrado."
            search-placeholder="Buscar usuário..."
            :search-fields="['username']"
            :page-size="15"
            :columns="[
                { key: 'username', label: 'Usuário', sortable: true },
                { key: 'roles', label: 'Papéis (Roles)' },
                { key: 'ativo', label: 'Situação', class: 'w-32' },
                ...(vm.isAdmin ? [{ key: 'acoes', label: 'Ações', class: 'w-40' }] : []),
            ]"
        >
            <template #cell-roles="{ row }">
                <div class="flex flex-wrap gap-1">
                    <StatusBadge
                        v-for="r in vm.parseRoles(row.roles)"
                        :key="r"
                        :value="ROLE_LABELS[r as RoleName] ?? r"
                        :severity="r === 'admin' ? 'danger' : 'info'"
                        class="text-[0.72rem]"
                    />
                </div>
            </template>
            <template #cell-ativo="{ row }">
                <StatusBadge :value="row.ativo ? 'Ativo' : 'Inativo'" :severity="row.ativo ? 'success' : 'secondary'" />
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button
                        v-if="row.ativo"
                        variant="ghost"
                        size="icon-sm"
                        title="Editar papéis"
                        @click="vm.abrirEdicao(row)"
                    >
                        <Pencil class="size-4" />
                    </Button>
                    <Button
                        v-if="row.ativo && row.username !== vm.username"
                        variant="ghost"
                        size="icon-sm"
                        class="text-destructive"
                        title="Desativar usuário"
                        @click="vm.confirmarDesativar(row)"
                    >
                        <X class="size-4" />
                    </Button>
                    <Button
                        v-if="!row.ativo"
                        variant="ghost"
                        size="icon-sm"
                        class="text-emerald-600"
                        title="Reativar usuário"
                        @click="vm.reativar(row)"
                    >
                        <RefreshCw class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <!-- Novo usuário -->
        <Dialog v-model:open="vm.novoVisible">
            <DialogContent class="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>Novo Usuário</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4">
                    <Field>
                        <FieldLabel for="nu-username">Usuário</FieldLabel>
                        <Input id="nu-username" v-model="vm.novoForm.username" placeholder="joao.silva" autocomplete="off" />
                    </Field>
                    <Field>
                        <FieldLabel for="nu-senha">Senha inicial</FieldLabel>
                        <PasswordInput id="nu-senha" v-model="vm.novoForm.senha" autocomplete="new-password" />
                    </Field>
                    <div class="flex flex-col gap-1">
                        <label class="mb-1 text-sm font-medium">Papéis</label>
                        <div class="grid grid-cols-2 gap-2">
                            <div v-for="[value, label] in vm.rolesOpcoes" :key="value" class="flex items-center gap-2">
                                <Checkbox
                                    :id="`role-${value}`"
                                    :model-value="vm.novoForm.roles.includes(value)"
                                    :disabled="value === 'admin' && !vm.novoForm.roles.includes('admin')"
                                    @update:model-value="(checked) => {
                                        if (checked) vm.novoForm.roles.push(value)
                                        else vm.novoForm.roles = vm.novoForm.roles.filter((r: string) => r !== value)
                                    }"
                                />
                                <label :for="`role-${value}`" class="cursor-pointer text-sm">{{ label }}</label>
                            </div>
                        </div>
                        <small class="mt-1 text-muted-foreground">Administradores têm acesso total e não precisam de outros papéis.</small>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.novoVisible = false">Cancelar</Button>
                    <Button
                        :disabled="!vm.novoForm.username || !vm.novoForm.senha || !vm.novoForm.roles.length"
                        @click="vm.criarUsuario"
                    >
                        <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Criar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Editar papéis -->
        <Dialog v-model:open="vm.edicaoVisible">
            <DialogContent class="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>Editar Papéis</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4">
                    <p class="text-muted-foreground">
                        Papéis do usuário <strong>{{ vm.usuarioEdicao?.username }}</strong>
                    </p>
                    <div class="grid grid-cols-2 gap-2">
                        <div v-for="[value, label] in vm.rolesOpcoes" :key="value" class="flex items-center gap-2">
                            <Checkbox
                                :id="`edit-role-${value}`"
                                :model-value="vm.edicaoRoles.includes(value)"
                                @update:model-value="(checked) => {
                                    if (checked) vm.edicaoRoles.push(value)
                                    else vm.edicaoRoles = vm.edicaoRoles.filter((r: string) => r !== value)
                                }"
                            />
                            <label :for="`edit-role-${value}`" class="cursor-pointer text-sm">{{ label }}</label>
                        </div>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.edicaoVisible = false">Cancelar</Button>
                    <Button :disabled="!vm.edicaoRoles.length" @click="vm.salvarEdicao">
                        <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Alterar minha senha -->
        <Dialog v-model:open="vm.senhaVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Alterar Senha</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4">
                    <Field>
                        <FieldLabel for="senha-atual">Senha atual</FieldLabel>
                        <PasswordInput id="senha-atual" v-model="vm.senhaForm.senha_atual" autocomplete="current-password" />
                    </Field>
                    <Field>
                        <FieldLabel for="nova-senha">Nova senha</FieldLabel>
                        <PasswordInput id="nova-senha" v-model="vm.senhaForm.nova_senha" autocomplete="new-password" />
                    </Field>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.senhaVisible = false">Cancelar</Button>
                    <Button
                        :disabled="!vm.senhaForm.senha_atual || !vm.senhaForm.nova_senha"
                        @click="vm.alterarSenha"
                    >
                        <LoaderCircle v-if="vm.salvandoSenha" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Alterar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Confirmar desativação -->
        <Dialog v-model:open="vm.desativarVisible">
            <DialogContent class="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>Desativar Usuário</DialogTitle>
                </DialogHeader>
                <p>
                    Tem certeza que deseja desativar o usuário <strong>{{ vm.usuarioDesativar?.username }}</strong>?
                    Ele não conseguirá mais fazer login.
                </p>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.desativarVisible = false">Cancelar</Button>
                    <Button variant="destructive" @click="vm.desativar">
                        <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                        <X v-else class="size-4" />
                        Desativar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
