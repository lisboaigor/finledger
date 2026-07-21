<script setup lang="ts">
definePageMeta({ layout: 'backoffice' })

import { Ban, Check, CheckCircle2, Copy, ExternalLink, LoaderCircle, Pencil, Plus, Search, Tag, User } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const vm = useTenantsViewModel()

onMounted(() => {
    vm.verificarAcesso()
    vm.carregar()
})
</script>

<template>
    <div>
        <div class="mb-6">
            <h1 class="text-2xl font-semibold">Tenants</h1>
            <p class="text-muted-foreground">Gestão de clientes e seus planos.</p>
        </div>

        <div class="mb-4 flex items-center justify-between gap-2 border-b pb-3">
            <InputGroup class="max-w-sm">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filters.global.value" placeholder="Buscar tenant..." />
            </InputGroup>
            <Button v-if="vm.hasPermission('tenants:write')" @click="vm.novoVisible = true">
                <Plus class="size-4" />
                Novo Tenant
            </Button>
        </div>

        <MessageBox v-if="!vm.canRead" severity="warn" class="mb-4">
            Você não tem a permissão <code>tenants:read</code>. Peça acesso a um superadmin.
        </MessageBox>

        <Card v-if="vm.canRead">
            <CardContent>
                <AppDataTable
                    :rows="vm.tenants"
                    :loading="vm.loading"
                    row-key="tenant_id"
                    empty-text="Nenhum tenant cadastrado."
                    search-placeholder="Buscar tenant..."
                    :search-fields="['slug', 'nome']"
                    :columns="[
                        { key: 'slug', label: 'Slug', class: 'w-48' },
                        { key: 'nome', label: 'Nome' },
                        { key: 'plano', label: 'Plano' },
                        { key: 'status', label: 'Status' },
                        { key: 'acoes', label: 'Ações', class: 'w-64' },
                    ]"
                >
                    <template #cell-plano="{ row }">
                        <StatusBadge :value="row.plano" severity="info" />
                    </template>
                    <template #cell-status="{ row }">
                        <StatusBadge :value="row.status" :severity="row.status === 'ativo' ? 'success' : 'danger'" />
                    </template>
                    <template #cell-acoes="{ row }">
                        <div class="flex gap-1">
                            <Button
                                v-if="vm.hasPermission('tenants:write')"
                                variant="ghost"
                                size="icon-sm"
                                title="Editar nome"
                                @click="vm.abrirEdicao(row)"
                            >
                                <Pencil class="size-4" />
                            </Button>
                            <Button
                                v-if="vm.hasPermission('tenants:write') && row.status === 'ativo'"
                                variant="ghost"
                                size="icon-sm"
                                class="text-amber-600"
                                title="Suspender"
                                @click="vm.suspender(row.tenant_id)"
                            >
                                <Ban class="size-4" />
                            </Button>
                            <Button
                                v-if="vm.hasPermission('tenants:write') && row.status === 'suspenso'"
                                variant="ghost"
                                size="icon-sm"
                                class="text-emerald-600"
                                title="Reativar"
                                @click="vm.reativar(row.tenant_id)"
                            >
                                <CheckCircle2 class="size-4" />
                            </Button>
                            <Button
                                v-if="vm.hasPermission('tenants:write')"
                                variant="ghost"
                                size="icon-sm"
                                title="Alterar plano"
                                @click="vm.abrirPlano(row)"
                            >
                                <Tag class="size-4" />
                            </Button>
                            <Button
                                v-if="vm.hasPermission('tenants:impersonate')"
                                variant="ghost"
                                size="icon-sm"
                                title="Acessar como tenant"
                                @click="vm.impersonar(row)"
                            >
                                <User class="size-4" />
                            </Button>
                        </div>
                    </template>
                </AppDataTable>
            </CardContent>
        </Card>

        <!-- Novo tenant -->
        <Dialog v-model:open="vm.novoVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Novo Tenant</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-3">
                    <Field>
                        <FieldLabel for="novo-slug">Slug</FieldLabel>
                        <Input id="novo-slug" v-model="vm.novoForm.slug" placeholder="minha-empresa" />
                    </Field>
                    <Field>
                        <FieldLabel for="novo-nome">Nome</FieldLabel>
                        <Input id="novo-nome" v-model="vm.novoForm.nome" placeholder="Minha Empresa Ltda." />
                    </Field>
                    <Field>
                        <FieldLabel for="novo-admin">Usuário admin inicial</FieldLabel>
                        <Input id="novo-admin" v-model="vm.novoForm.admin_username" placeholder="admin" />
                    </Field>
                    <Field>
                        <FieldLabel for="novo-senha">Senha admin</FieldLabel>
                        <PasswordInput id="novo-senha" v-model="vm.novoForm.admin_senha" class="w-full" />
                    </Field>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.novoVisible = false">Cancelar</Button>
                    <Button @click="vm.criarTenant">
                        <LoaderCircle v-if="vm.saving" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Criar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Editar tenant -->
        <Dialog v-model:open="vm.edicaoVisible">
            <DialogContent class="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>Editar Tenant</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel for="edicao-nome">Nome</FieldLabel>
                    <Input id="edicao-nome" v-model="vm.edicaoForm.nome" />
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.edicaoVisible = false">Cancelar</Button>
                    <Button :disabled="!vm.edicaoForm.nome.trim()" @click="vm.salvarEdicao">
                        <LoaderCircle v-if="vm.saving" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Alterar plano -->
        <Dialog v-model:open="vm.planoVisible">
            <DialogContent class="sm:max-w-xs">
                <DialogHeader>
                    <DialogTitle>Alterar Plano</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel for="plano-select">Plano</FieldLabel>
                    <Select v-model="vm.planoForm.plano">
                        <SelectTrigger id="plano-select" class="w-full">
                            <SelectValue placeholder="Selecione" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem v-for="o in vm.planosOpcoes" :key="o.value" :value="o.value">
                                {{ o.label }}
                            </SelectItem>
                        </SelectContent>
                    </Select>
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.planoVisible = false">Cancelar</Button>
                    <Button @click="vm.salvarPlano">
                        <LoaderCircle v-if="vm.saving" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Token de impersonation -->
        <Dialog v-model:open="vm.impersonarVisible">
            <DialogContent class="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>Acesso como Tenant</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-3">
                    <MessageBox severity="warn">
                        Este acesso expira em 1 hora. Use-o para suporte técnico apenas.
                    </MessageBox>
                    <div class="flex flex-col gap-1">
                        <label class="text-sm font-medium">Link de acesso ao tenant</label>
                        <InputGroup>
                            <InputGroupInput :model-value="vm.impersonarUrl" readonly class="font-mono text-xs" />
                            <InputGroupAddon align="inline-end">
                                <Button variant="ghost" size="icon-sm" title="Copiar link" @click="vm.copiarToken">
                                    <Copy class="size-4" />
                                </Button>
                            </InputGroupAddon>
                        </InputGroup>
                    </div>
                    <p class="text-sm text-muted-foreground">
                        Abra o link em uma nova aba para entrar no tenant já autenticado como suporte.
                    </p>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.impersonarVisible = false">Fechar</Button>
                    <Button @click="vm.abrirImpersonacao">
                        <ExternalLink class="size-4" />
                        Abrir tenant
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
