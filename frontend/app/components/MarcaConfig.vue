<script setup lang="ts">
import { Check, LoaderCircle, Palette, RotateCcw, Trash2, Upload } from '@lucide/vue'
import type { Marca } from '~/models/marca'
import { atualizarMarca, familiaDaFonte, FONTES_MARCA, TAMANHO_MARCA_MAX, TAMANHO_MARCA_MIN, TAMANHO_MARCA_PADRAO } from '~/models/marca'
import { Button } from '@/components/ui/button'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

// Cor inicial do seletor quando ainda no padrão (só vira valor enviado se o
// admin de fato personalizar).
const COR_PRIMARIA_PADRAO = '#0d9271'
const COR_FONTE_PADRAO = '#0a0a0a'
const LOGO_MAX_PX = 256

const { apiFetch, apiErrorMessage } = useApi()
const { notifySuccess, notifyError } = useNotify()
const { marca, nome: marcaNome, carregar, previsualizar, restaurar, aplicar } = useMarca()

// Cópia editável — o estado global só muda ao salvar.
const form = reactive<Marca>({ ...marca.value })
const salvando = ref(false)
const logoInput = ref<HTMLInputElement | null>(null)
let observando = false

// Prévia do wordmark: reflete fonte, tamanho (escala sobre um base fixo da
// prévia) e cor escolhidos — igual ao que o `.brand-wordmark` faz no app.
const previewFamilia = computed(() => familiaDaFonte(form.marca_fonte) || "'Grand Hotel', cursive")
const tamanhoAtual = computed(() => form.marca_fonte_tamanho ?? TAMANHO_MARCA_PADRAO)
const previewStyle = computed(() => ({
    fontFamily: previewFamilia.value,
    fontSize: `calc(1.75rem * ${tamanhoAtual.value / 100})`,
    color: form.marca_fonte_cor || undefined,
}))

// Garante que o form reflita a marca já carregada (o plugin é assíncrono; num
// acesso direto a /configuracoes ela pode ainda não ter chegado). Só depois
// disso liga a prévia ao vivo, para não pré-visualizar um estado transitório.
onMounted(async () => {
    await carregar()
    Object.assign(form, marca.value)
    await nextTick()
    observando = true
})

// Prévia ao vivo: qualquer mudança reflete no app inteiro sem persistir.
watch(form, () => { if (observando) previsualizar(form) }, { deep: true })
onBeforeUnmount(() => restaurar())

function abrirSeletorLogo() {
    logoInput.value?.click()
}

async function aoEscolherLogo(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (!file) return
    if (!file.type.startsWith('image/')) {
        notifyError('Selecione um arquivo de imagem.')
        return
    }
    try {
        form.marca_logo_data_uri = await redimensionarParaDataUri(file)
    } catch {
        notifyError('Não foi possível processar a imagem.')
    } finally {
        if (logoInput.value) logoInput.value.value = ''
    }
}

/** Redimensiona a imagem para caber em LOGO_MAX_PX (mantendo proporção) e
 * devolve um data URI PNG — evita guardar imagens grandes no banco. */
function redimensionarParaDataUri(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
        const img = new Image()
        img.onload = () => {
            const escala = Math.min(1, LOGO_MAX_PX / Math.max(img.width, img.height))
            const w = Math.round(img.width * escala)
            const h = Math.round(img.height * escala)
            const canvas = document.createElement('canvas')
            canvas.width = w
            canvas.height = h
            const ctx = canvas.getContext('2d')
            if (!ctx) return reject(new Error('sem canvas'))
            ctx.drawImage(img, 0, 0, w, h)
            resolve(canvas.toDataURL('image/png'))
        }
        img.onerror = () => reject(new Error('imagem inválida'))
        img.src = URL.createObjectURL(file)
    })
}

function removerLogo() {
    form.marca_logo_data_uri = null
}

function restaurarTudo() {
    form.marca_nome = null
    form.marca_logo_data_uri = null
    form.marca_cor_primaria = null
    form.marca_fonte = null
    form.marca_fonte_tamanho = null
    form.marca_fonte_cor = null
}

async function salvar() {
    salvando.value = true
    try {
        const payload: Marca = {
            marca_nome: form.marca_nome?.trim() || null,
            marca_logo_data_uri: form.marca_logo_data_uri || null,
            marca_cor_primaria: form.marca_cor_primaria || null,
            marca_fonte: form.marca_fonte || null,
            marca_fonte_tamanho: form.marca_fonte_tamanho ?? null,
            marca_fonte_cor: form.marca_fonte_cor || null,
        }
        await atualizarMarca(apiFetch, payload)
        marca.value = { ...payload }
        aplicar(marca.value)
        notifySuccess('Identidade visual salva', 'As mudanças já valem para todos os usuários.')
    } catch (e) {
        notifyError(apiErrorMessage(e))
    } finally {
        salvando.value = false
    }
}
</script>

<template>
    <AppFieldset legend="Identidade visual">
        <template #legend>
            <span class="flex items-center gap-2">
                <Palette class="size-4" />
                <span>Identidade visual</span>
            </span>
        </template>

        <p class="mb-4 text-sm text-muted-foreground">
            Personalize a marca da sua empresa: nome, logo, cor de destaque e a fonte, o tamanho e a
            cor do nome. As mudanças aparecem para todos os usuários e também na tela de login. Deixe
            em branco para usar o padrão Finledger.
        </p>

        <div class="flex flex-col gap-4">
            <Field class="max-w-md">
                <FieldLabel>Nome exibido</FieldLabel>
                <Input v-model="form.marca_nome" maxlength="40" placeholder="Finledger" />
            </Field>

            <!-- Logo -->
            <div class="flex flex-col gap-2">
                <span class="text-sm font-medium">Logo</span>
                <div class="flex items-center gap-4">
                    <div class="flex size-16 items-center justify-center rounded-lg border bg-muted/40">
                        <img v-if="form.marca_logo_data_uri" :src="form.marca_logo_data_uri" alt="Prévia do logo" class="max-h-12 max-w-12 object-contain">
                        <AppLogoIcon v-else class="text-primary" style="font-size: 2rem" />
                    </div>
                    <div class="flex flex-col gap-2">
                        <input ref="logoInput" type="file" accept="image/*" class="hidden" @change="aoEscolherLogo">
                        <div class="flex gap-2">
                            <Button variant="outline" size="sm" @click="abrirSeletorLogo">
                                <Upload class="size-4" />
                                Enviar imagem
                            </Button>
                            <Button v-if="form.marca_logo_data_uri" variant="ghost" size="sm" class="text-destructive" @click="removerLogo">
                                <Trash2 class="size-4" />
                                Remover
                            </Button>
                        </div>
                        <span class="text-xs text-muted-foreground">PNG ou SVG, fundo transparente. Redimensionado automaticamente.</span>
                    </div>
                </div>
            </div>

            <!-- Fonte do nome -->
            <div class="flex flex-col gap-2 max-w-md">
                <span class="text-sm font-medium">Fonte do nome</span>
                <select
                    class="border-input bg-background h-9 w-full rounded-lg border px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-3"
                    :value="form.marca_fonte || ''"
                    @change="form.marca_fonte = ($event.target as HTMLSelectElement).value || null"
                >
                    <option value="">Padrão (Grand Hotel)</option>
                    <option v-for="f in FONTES_MARCA" :key="f.chave" :value="f.chave">{{ f.rotulo }}</option>
                </select>
            </div>

            <!-- Tamanho do nome -->
            <div class="flex flex-col gap-2 max-w-md">
                <span class="text-sm font-medium">Tamanho do nome</span>
                <div class="flex items-center gap-3">
                    <input
                        type="range"
                        :min="TAMANHO_MARCA_MIN"
                        :max="TAMANHO_MARCA_MAX"
                        step="5"
                        :value="tamanhoAtual"
                        class="accent-primary h-2 flex-1 cursor-pointer"
                        aria-label="Tamanho do nome em porcentagem"
                        @input="form.marca_fonte_tamanho = Number(($event.target as HTMLInputElement).value)"
                    >
                    <span class="w-12 text-right font-mono text-sm text-muted-foreground">{{ tamanhoAtual }}%</span>
                    <Button v-if="form.marca_fonte_tamanho != null" variant="ghost" size="sm" @click="form.marca_fonte_tamanho = null">
                        Padrão
                    </Button>
                </div>
            </div>

            <!-- Cor do nome -->
            <div class="flex flex-col gap-2 max-w-md">
                <span class="text-sm font-medium">Cor do nome</span>
                <div class="flex items-center gap-2">
                    <input
                        type="color"
                        class="h-9 w-14 shrink-0 cursor-pointer rounded-lg border bg-background p-1"
                        :value="form.marca_fonte_cor || COR_FONTE_PADRAO"
                        @input="form.marca_fonte_cor = ($event.target as HTMLInputElement).value"
                    >
                    <span class="font-mono text-sm text-muted-foreground">{{ form.marca_fonte_cor || 'padrão' }}</span>
                    <Button v-if="form.marca_fonte_cor" variant="ghost" size="sm" class="ml-auto" @click="form.marca_fonte_cor = null">
                        Padrão
                    </Button>
                </div>
            </div>

            <!-- Prévia do nome: fonte + tamanho + cor -->
            <div class="max-w-md rounded-lg border bg-muted/40 px-3 py-3">
                <span class="text-xs text-muted-foreground">Prévia:</span>
                <span class="ml-2 align-middle" :style="previewStyle">{{ marcaNome }}</span>
            </div>

            <!-- Cor de destaque -->
            <div class="flex flex-col gap-2 max-w-md">
                <span class="text-sm font-medium">Cor de destaque</span>
                <div class="flex items-center gap-2">
                    <input
                        type="color"
                        class="h-9 w-14 shrink-0 cursor-pointer rounded-lg border bg-background p-1"
                        :value="form.marca_cor_primaria || COR_PRIMARIA_PADRAO"
                        @input="form.marca_cor_primaria = ($event.target as HTMLInputElement).value"
                    >
                    <span class="font-mono text-sm text-muted-foreground">{{ form.marca_cor_primaria || 'padrão' }}</span>
                    <Button v-if="form.marca_cor_primaria" variant="ghost" size="sm" class="ml-auto" @click="form.marca_cor_primaria = null">
                        Padrão
                    </Button>
                </div>
                <span class="text-xs text-muted-foreground">Botões, links, itens ativos e gráficos.</span>
            </div>

            <div class="flex flex-wrap gap-2 pt-1">
                <Button :disabled="salvando" @click="salvar">
                    <LoaderCircle v-if="salvando" class="size-4 animate-spin" />
                    <Check v-else class="size-4" />
                    Salvar identidade visual
                </Button>
                <Button variant="ghost" @click="restaurarTudo">
                    <RotateCcw class="size-4" />
                    Restaurar tudo ao padrão
                </Button>
            </div>
        </div>
    </AppFieldset>
</template>
