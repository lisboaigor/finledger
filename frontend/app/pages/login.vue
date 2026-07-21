<script setup lang="ts">
definePageMeta({ layout: false })

import { Building2, Lock, LoaderCircle, LogIn } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

const {
    isBackoffice,
    tenantSlug,
    loading,
    error,
    boForm,
    handleBackofficeLogin,
    tenantForm,
    handleTenantLogin,
    verificarSessao,
} = useLoginViewModel()

const { nome: marcaNome, logoDataUri } = useMarca()

onMounted(() => {
    verificarSessao()
})
</script>

<template>
    <div class="login-page">
        <div class="login-wrapper">

            <!-- Logo -->
            <div class="login-logo">
                <div v-if="logoDataUri" class="login-logo-custom">
                    <AppLogoIcon />
                </div>
                <div v-else class="login-logo-icon">
                    <AppLogoIcon />
                </div>
                <span class="login-logo-text brand-wordmark">{{ marcaNome }}</span>
            </div>

            <!-- Card -->
            <Card class="login-card">
                <CardHeader>
                    <CardTitle class="text-xl">{{ isBackoffice ? 'Painel Administrativo' : 'Acesso ao Sistema' }}</CardTitle>
                    <CardDescription>
                        <span v-if="isBackoffice" class="inline-flex items-center gap-1"><Lock class="size-3.5" />Acesso restrito a administradores</span>
                        <span v-else-if="tenantSlug" class="inline-flex items-center gap-1"><Building2 class="size-3.5" />{{ tenantSlug }}</span>
                        <span v-else>Informe suas credenciais para continuar</span>
                    </CardDescription>
                </CardHeader>
                <CardContent>
                    <!-- Backoffice -->
                    <form v-if="isBackoffice" class="login-form" @submit.prevent="handleBackofficeLogin">
                        <Field>
                            <FieldLabel for="bo-username">Usuário</FieldLabel>
                            <Input
                                id="bo-username"
                                v-model="boForm.username"
                                :disabled="loading"
                                autofocus
                                autocomplete="username"
                                class="w-full"
                            />
                        </Field>
                        <Field>
                            <FieldLabel for="bo-senha">Senha</FieldLabel>
                            <PasswordInput
                                id="bo-senha"
                                v-model="boForm.senha"
                                :disabled="loading"
                                autocomplete="current-password"
                                class="w-full"
                            />
                        </Field>
                        <MessageBox v-if="error" severity="error" class="login-error">{{ error }}</MessageBox>
                        <Button type="submit" :disabled="loading" size="lg" class="w-full">
                            <LoaderCircle v-if="loading" class="size-4 animate-spin" />
                            <LogIn v-else class="size-4" />
                            Entrar
                        </Button>
                    </form>

                    <!-- Tenant -->
                    <form v-else class="login-form" @submit.prevent="handleTenantLogin">
                        <Field>
                            <FieldLabel for="tenant-usuario">Usuário</FieldLabel>
                            <Input
                                id="tenant-usuario"
                                v-model="tenantForm.username"
                                :disabled="loading"
                                autofocus
                                autocomplete="username"
                                class="w-full"
                            />
                        </Field>
                        <Field>
                            <FieldLabel for="tenant-senha">Senha</FieldLabel>
                            <PasswordInput
                                id="tenant-senha"
                                v-model="tenantForm.senha"
                                :disabled="loading"
                                autocomplete="current-password"
                                class="w-full"
                            />
                        </Field>
                        <MessageBox v-if="error" severity="error" class="login-error">{{ error }}</MessageBox>
                        <Button type="submit" :disabled="loading" size="lg" class="w-full">
                            <LoaderCircle v-if="loading" class="size-4 animate-spin" />
                            <LogIn v-else class="size-4" />
                            Entrar
                        </Button>
                    </form>
                </CardContent>
            </Card>

        </div>
    </div>
</template>

<style scoped>
.login-page {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--background);
}

.login-wrapper {
    width: 100%;
    max-width: 24rem;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
}

/* ── Logo ── */
.login-logo {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.login-logo-icon {
    width: 3.5rem;
    height: 3.5rem;
    border-radius: 0.75rem;
    background: var(--primary);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--primary-foreground);
    font-size: 2.25rem;
}

/* Só define o tamanho base; tamanho final e cor vêm do `.brand-wordmark`
   (escala/cor do whitelabel). */
.login-logo-text {
    --brand-wordmark-base: 2.5rem;
}

/* Logo personalizado do tenant: sem o quadrado colorido, imagem maior. */
.login-logo-custom {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 3rem;
    color: var(--primary);
}

/* ── Card ── */
.login-card {
    width: 100%;
    box-shadow: 0 4px 24px 0 rgba(0, 0, 0, 0.06);
}

/* ── Form ── */
.login-form {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding-top: 0.5rem;
}

.login-error {
    margin: 0;
}
</style>
