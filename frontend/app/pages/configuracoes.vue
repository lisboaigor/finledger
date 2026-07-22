<script setup lang="ts">
import { Box, Building2, Check, Landmark, LoaderCircle, Pencil, Tag, Trash2, TrendingUp } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'

const vm = useConfiguracoesViewModel()
const { formatCentavos } = useFormat()

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Configurações</h1>
            <p class="text-muted-foreground">Regras de negócio deste comércio.</p>
        </div>

        <div v-if="vm.loading" class="py-8 text-center">
            <LoaderCircle class="mx-auto size-10 animate-spin text-muted-foreground" />
        </div>

        <MarcaConfig v-if="!vm.loading && vm.isAdmin" class="mb-4" />

        <AppFieldset v-if="!vm.loading" legend="Estoque">
            <template #legend>
                <span class="flex items-center gap-2">
                    <Box class="size-4" />
                    <span>Estoque</span>
                </span>
            </template>

            <div class="flex max-w-2xl items-start gap-3">
                <Checkbox
                    id="permite-orcamento-sem-estoque"
                    v-model="vm.permiteOrcamentoSemEstoque"
                    :disabled="!vm.isAdmin"
                />
                <label for="permite-orcamento-sem-estoque" class="text-sm" :class="{ 'cursor-pointer': vm.isAdmin }">
                    <span class="block font-medium">Permitir orçamento sem estoque</span>
                    <span class="text-muted-foreground">
                        Quando ligado, o vendedor pode incluir num orçamento produtos com saldo insuficiente
                        (ex.: para negociar prazo de reposição com o cliente). Quando desligado, o sistema bloqueia
                        e exige saldo disponível — o vendedor sempre vê a quantidade em estoque ao escolher o produto.
                        Em vendas, o mesmo bloqueio já vale por padrão, com a opção de confirmar "venda sob encomenda"
                        item a item.
                    </span>
                </label>
            </div>

            <p v-if="!vm.isAdmin" class="mt-3 text-sm text-muted-foreground">
                Apenas administradores podem alterar esta configuração.
            </p>

            <Button v-if="vm.isAdmin" class="mt-4 self-start" :disabled="vm.salvando" @click="vm.salvar">
                <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                <Check v-else class="size-4" />
                Salvar
            </Button>
        </AppFieldset>

        <AppFieldset v-if="!vm.loading" legend="Limpeza automática (lixeira)" class="mt-4">
            <template #legend>
                <span class="flex items-center gap-2">
                    <Trash2 class="size-4" />
                    <span>Limpeza automática (lixeira)</span>
                </span>
            </template>

            <div class="flex max-w-2xl flex-col gap-2">
                <label for="arquivamento-dias" class="text-sm font-medium">
                    Arquivar vendas e orçamentos não concretizados após
                </label>
                <div class="flex w-full items-center gap-2 sm:w-48">
                    <Input
                        id="arquivamento-dias"
                        v-model.number="vm.arquivamentoDias"
                        type="number"
                        min="1"
                        :disabled="!vm.isAdmin"
                    />
                    <span class="shrink-0 text-sm text-muted-foreground">dias</span>
                </div>
                <small class="text-muted-foreground">
                    Vendas abandonadas/canceladas e orçamentos que não viraram venda (rascunhos antigos,
                    recusados, expirados, cancelados) somem das listagens após esse prazo — mas
                    <strong>nada é excluído</strong>: ficam na Lixeira de cada tela, de onde um
                    administrador pode restaurar. Vazio = limpeza desligada.
                </small>
            </div>

            <Button v-if="vm.isAdmin" class="mt-4 self-start" :disabled="vm.salvando" @click="vm.salvar">
                <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                <Check v-else class="size-4" />
                Salvar
            </Button>
        </AppFieldset>

        <AppFieldset v-if="!vm.loading" legend="Dados da empresa" class="mt-4">
            <template #legend>
                <span class="flex items-center gap-2">
                    <Building2 class="size-4" />
                    <span>Dados da empresa</span>
                </span>
            </template>

            <p class="mb-4 text-sm text-muted-foreground">
                Aparecem nas impressões de venda e orçamento. Deixe em branco o que não se aplica.
            </p>

            <div class="flex max-w-2xl flex-col gap-4">
                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                    <div class="flex flex-col gap-2">
                        <label for="cnpj" class="text-sm font-medium">CNPJ</label>
                        <Input id="cnpj" v-model="vm.cnpj" :disabled="!vm.isAdmin" />
                    </div>
                    <div class="flex flex-col gap-2">
                        <label for="telefone" class="text-sm font-medium">Telefone</label>
                        <Input id="telefone" v-model="vm.telefone" :disabled="!vm.isAdmin" />
                    </div>
                </div>

                <div class="flex flex-col gap-2">
                    <label for="endereco" class="text-sm font-medium">Endereço</label>
                    <Input id="endereco" v-model="vm.endereco" :disabled="!vm.isAdmin" />
                </div>

                <div class="flex flex-col gap-2">
                    <label for="chave-pix" class="text-sm font-medium">Chave PIX</label>
                    <Input id="chave-pix" v-model="vm.chavePix" :disabled="!vm.isAdmin" />
                </div>

                <div class="flex flex-col gap-2">
                    <label for="info-adicional" class="text-sm font-medium">Informações adicionais</label>
                    <Textarea
                        id="info-adicional"
                        v-model="vm.informacoesAdicionais"
                        rows="3"
                        :disabled="!vm.isAdmin"
                        placeholder="Ex.: Instagram, WhatsApp, horário de funcionamento…"
                    />
                </div>
            </div>

            <p v-if="!vm.isAdmin" class="mt-3 text-sm text-muted-foreground">
                Apenas administradores podem alterar estes dados.
            </p>

            <Button v-if="vm.isAdmin" class="mt-4 self-start" :disabled="vm.salvando" @click="vm.salvar">
                <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                <Check v-else class="size-4" />
                Salvar
            </Button>
        </AppFieldset>

        <AppFieldset v-if="!vm.loading" legend="Perfil fiscal" class="mt-4">
            <template #legend>
                <span class="flex items-center gap-2">
                    <Landmark class="size-4" />
                    <span>Perfil fiscal</span>
                </span>
            </template>

            <p class="mb-4 max-w-2xl text-sm text-muted-foreground">
                Determina como os impostos das notas fiscais são calculados, já seguindo a transição da
                reforma tributária (CBS/IBS). Sem regime configurado, o sistema mantém o comportamento
                padrão (Simples Nacional em SP). Com regime preenchido, UF, município e CRT são obrigatórios.
            </p>

            <div class="flex max-w-2xl flex-col gap-4">
                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                    <div class="flex flex-col gap-2">
                        <label for="regime-select" class="text-sm font-medium">Regime tributário</label>
                        <Select v-model="vm.perfilRegime" :disabled="!vm.isAdmin">
                            <SelectTrigger id="regime-select" class="w-full">
                                <SelectValue placeholder="Padrão (não configurado)" />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem v-for="o in vm.regimesTributarios" :key="o.value" :value="o.value">
                                    {{ o.label }}
                                </SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                    <div class="flex flex-col gap-2">
                        <label for="crt-select" class="text-sm font-medium">CRT (código de regime na NF-e)</label>
                        <Select v-model="vm.perfilCrt" :disabled="!vm.isAdmin">
                            <SelectTrigger id="crt-select" class="w-full">
                                <SelectValue placeholder="Selecione" />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem v-for="o in vm.opcoesCrt" :key="o.value" :value="o.value">
                                    {{ o.label }}
                                </SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                </div>

                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                    <div class="flex flex-col gap-2">
                        <label for="uf-select" class="text-sm font-medium">UF</label>
                        <Select v-model="vm.perfilUf" :disabled="!vm.isAdmin">
                            <SelectTrigger id="uf-select" class="w-full">
                                <SelectValue placeholder="Selecione" />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem v-for="uf in vm.ufs" :key="uf" :value="uf">{{ uf }}</SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                    <div class="flex flex-col gap-2">
                        <label for="municipio-ibge" class="text-sm font-medium">Município (código IBGE, 7 dígitos)</label>
                        <Input
                            id="municipio-ibge"
                            v-model="vm.perfilMunicipio"
                            :disabled="!vm.isAdmin"
                            placeholder="Ex.: 3550308 (São Paulo)"
                        />
                    </div>
                </div>

                <div v-if="vm.perfilRegime === 'simples_nacional'" class="flex flex-col gap-2">
                    <label for="das-pct" class="text-sm font-medium">Alíquota efetiva do DAS (%)</label>
                    <InputPercent
                        id="das-pct"
                        v-model="vm.perfilDasPct"
                        :min="0"
                        :max="30"
                        :max-fraction-digits="2"
                        :disabled="!vm.isAdmin"
                        class="w-full sm:w-48"
                    />
                    <small class="text-muted-foreground">
                        Alíquota efetiva do seu anexo/faixa do Simples. Vira o custo tributário
                        usado na precificação — no Simples a nota não destaca ICMS/PIS/COFINS,
                        o recolhimento acontece dentro do DAS.
                    </small>
                </div>

                <div class="flex items-start gap-3">
                    <Checkbox id="ibs-cbs-regular" v-model="vm.perfilIbsCbsRegular" :disabled="!vm.isAdmin" />
                    <label for="ibs-cbs-regular" class="text-sm" :class="{ 'cursor-pointer': vm.isAdmin }">
                        <span class="block font-medium">Optante pelo regime regular de IBS/CBS</span>
                        <span class="text-muted-foreground">
                            Só se aplica ao Simples Nacional: a opção pelo regime regular (LC 214/2025) permite
                            que seus clientes aproveitem crédito de IBS/CBS. Sem a opção, os valores destacados
                            na nota são informativos e o recolhimento segue dentro do DAS.
                        </span>
                    </label>
                </div>
            </div>

            <p v-if="!vm.isAdmin" class="mt-3 text-sm text-muted-foreground">
                Apenas administradores podem alterar o perfil fiscal.
            </p>

            <Button
                v-if="vm.isAdmin"
                class="mt-4 self-start"
                :disabled="vm.salvandoPerfilFiscal"
                @click="vm.salvarPerfilFiscal"
            >
                <LoaderCircle v-if="vm.salvandoPerfilFiscal" class="size-4 animate-spin" />
                <Check v-else class="size-4" />
                Salvar
            </Button>
        </AppFieldset>

        <AppFieldset v-if="!vm.loading" legend="Preço de venda" class="mt-4">
            <template #legend>
                <span class="flex items-center gap-2">
                    <Tag class="size-4" />
                    <span>Preço de venda</span>
                </span>
            </template>

            <p class="mb-4 text-sm text-muted-foreground">
                Com isso preenchido, o sistema sugere o preço de venda ao cadastrar produtos e ao
                registrar entradas de estoque. A sugestão nunca é aplicada sozinha — você sempre decide.
            </p>

            <div class="flex max-w-2xl flex-col gap-4">
                <div class="flex flex-col gap-2">
                    <label for="margem-padrao" class="text-sm font-medium">
                        Quanto você quer que SOBRE de cada venda, no final de tudo (%)
                    </label>
                    <InputPercent
                        id="margem-padrao"
                        v-model="vm.margemPadraoPct"
                        :min="0"
                        :max="99"
                        :max-fraction-digits="2"
                        :disabled="!vm.isAdmin"
                        class="w-full sm:w-48"
                    />
                    <small class="text-muted-foreground">
                        Atenção: esta <strong>não</strong> é a diferença entre o preço e o custo do produto
                        (a "margem de balcão", que costuma ser 40% ou mais). É a sobra <strong>final</strong>,
                        depois que a venda também paga sua parte dos custos fixos, cartão e demais percentuais.
                        Numa loja saudável, uma margem de balcão de 40% costuma deixar entre 5% e 15% de sobra
                        final — números pequenos aqui são normais. O passo a passo em cada produto mostra a
                        conta inteira.
                    </small>
                </div>

                <div>
                    <label class="mb-1 block text-sm font-medium">O que sai do preço antes de virar lucro (%)</label>
                    <small class="mb-2 block text-muted-foreground">
                        Preencha só o que se aplica à sua loja. Em "Outros" cabem perdas que acontecem em
                        toda loja e ninguém lança: calote esperado nas vendas a prazo (inadimplência) e
                        quebra/extravio de estoque — 1% a 2% costuma ser realista.
                    </small>
                    <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
                        <div class="flex flex-col gap-1">
                            <label for="pct-imposto" class="text-xs text-muted-foreground">Imposto</label>
                            <InputPercent id="pct-imposto" v-model="vm.impostoPct" :min="0" :max="99" :max-fraction-digits="2" :disabled="!vm.isAdmin" />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="pct-comissao" class="text-xs text-muted-foreground">Comissão de venda</label>
                            <InputPercent id="pct-comissao" v-model="vm.comissaoPct" :min="0" :max="99" :max-fraction-digits="2" :disabled="!vm.isAdmin" />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="pct-cartao" class="text-xs text-muted-foreground">Taxa de cartão</label>
                            <InputPercent id="pct-cartao" v-model="vm.cartaoPct" :min="0" :max="99" :max-fraction-digits="2" :disabled="!vm.isAdmin" />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="pct-frete" class="text-xs text-muted-foreground">Frete</label>
                            <InputPercent id="pct-frete" v-model="vm.fretePct" :min="0" :max="99" :max-fraction-digits="2" :disabled="!vm.isAdmin" />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="pct-outros" class="text-xs text-muted-foreground">Outros</label>
                            <InputPercent id="pct-outros" v-model="vm.outrosPct" :min="0" :max="99" :max-fraction-digits="2" :disabled="!vm.isAdmin" />
                        </div>
                    </div>
                </div>

                <MessageBox v-if="vm.percentuaisInvalidos" severity="warn">
                    Os percentuais somam 100% ou mais — assim não sobra preço. Reduza a margem ou os descontos.
                </MessageBox>

                <!-- Máquinas de cartão -->
                <div>
                    <label class="mb-1 block text-sm font-medium">Máquinas de cartão</label>
                    <small class="mb-2 block text-muted-foreground">
                        Cadastre suas máquinas e as taxas de cada uma. A sugestão de preço usa a maior
                        taxa — assim o lucro fecha mesmo quando a venda cai na máquina mais cara. Se
                        nenhuma for cadastrada, vale a "Taxa de cartão" única acima.
                    </small>

                    <AppDataTable
                        v-if="vm.maquinas.length"
                        :rows="vm.maquinas"
                        row-key="nome"
                        class="mb-3"
                        :page-size-options="[]"
                        :columns="[
                            { key: 'nome', label: 'Máquina' },
                            { key: 'taxa', label: 'Taxa' },
                            ...(vm.isAdmin ? [{ key: 'acoes', label: '', class: 'w-24' }] : []),
                        ]"
                    >
                        <template #cell-taxa="{ row }">{{ row.taxa_bps / 100 }}%</template>
                        <template #cell-acoes="{ row }">
                            <div class="flex gap-1">
                                <Button variant="ghost" size="icon-sm" @click="vm.editarMaquina(row)">
                                    <Pencil class="size-4" />
                                </Button>
                                <Button variant="ghost" size="icon-sm" class="text-destructive" @click="vm.removerMaquina(row.nome)">
                                    <Trash2 class="size-4" />
                                </Button>
                            </div>
                        </template>
                    </AppDataTable>

                    <div v-if="vm.isAdmin" class="flex flex-wrap items-end gap-3">
                        <div class="flex flex-col gap-1">
                            <label for="maquina-nome" class="text-xs text-muted-foreground">Máquina</label>
                            <Input id="maquina-nome" v-model="vm.novaMaquina.nome" placeholder="Ex.: Stone, Cielo…" class="w-48" />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="maquina-taxa" class="text-xs text-muted-foreground">Taxa (%)</label>
                            <InputPercent id="maquina-taxa" v-model="vm.novaMaquina.taxaPct" :min="0" :max="99" :max-fraction-digits="2" class="w-32" />
                        </div>
                        <Button
                            size="sm"
                            :disabled="!vm.novaMaquina.nome || vm.novaMaquina.taxaPct == null || vm.salvandoMaquina"
                            @click="vm.salvarMaquina"
                        >
                            <LoaderCircle v-if="vm.salvandoMaquina" class="size-4 animate-spin" />
                            <Check v-else class="size-4" />
                            Salvar máquina
                        </Button>
                    </div>
                </div>

                <!-- Exceções por categoria -->
                <div>
                    <label class="mb-1 block text-sm font-medium">Exceções por categoria</label>
                    <small class="mb-2 block text-muted-foreground">
                        Categorias listadas aqui usam a própria margem (e, se preenchido, o próprio custo
                        fixo por unidade) em vez do padrão da loja.
                    </small>

                    <AppDataTable
                        v-if="vm.margens.length"
                        :rows="vm.margens"
                        row-key="categoria"
                        class="mb-3"
                        :page-size-options="[]"
                        :columns="[
                            { key: 'categoria', label: 'Categoria' },
                            { key: 'ganha', label: 'Quer ganhar' },
                            { key: 'custo_fixo', label: 'Custo fixo por unidade' },
                            ...(vm.isAdmin ? [{ key: 'acoes', label: '', class: 'w-24' }] : []),
                        ]"
                    >
                        <template #cell-ganha="{ row }">{{ row.margem_bps / 100 }}%</template>
                        <template #cell-custo_fixo="{ row }">
                            {{ row.custo_fixo_unitario_centavos != null ? formatCentavos(row.custo_fixo_unitario_centavos) : '—' }}
                        </template>
                        <template #cell-acoes="{ row }">
                            <div class="flex gap-1">
                                <Button variant="ghost" size="icon-sm" @click="vm.editarMargem(row)">
                                    <Pencil class="size-4" />
                                </Button>
                                <Button variant="ghost" size="icon-sm" class="text-destructive" @click="vm.removerMargem(row.categoria)">
                                    <Trash2 class="size-4" />
                                </Button>
                            </div>
                        </template>
                    </AppDataTable>

                    <div v-if="vm.isAdmin" class="flex flex-wrap items-end gap-3">
                        <div class="flex flex-col gap-1">
                            <label for="margem-cat" class="text-xs text-muted-foreground">Categoria</label>
                            <Input
                                id="margem-cat"
                                v-model="vm.novaMargem.categoria"
                                list="categorias-sugeridas"
                                class="w-48"
                            />
                            <datalist id="categorias-sugeridas">
                                <option v-for="c in vm.categorias" :key="c" :value="c" />
                            </datalist>
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="margem-pct" class="text-xs text-muted-foreground">Quer ganhar (%)</label>
                            <InputPercent id="margem-pct" v-model="vm.novaMargem.margemPct" :min="0" :max="99" :max-fraction-digits="2" class="w-32" />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="margem-fixo" class="text-xs text-muted-foreground">Custo fixo/unidade (opcional)</label>
                            <InputMoney id="margem-fixo" v-model="vm.novaMargem.custoFixoUnitario" class="w-40" />
                        </div>
                        <Button
                            size="sm"
                            :disabled="!vm.novaMargem.categoria || vm.novaMargem.margemPct == null || vm.salvandoMargem"
                            @click="vm.salvarMargem"
                        >
                            <LoaderCircle v-if="vm.salvandoMargem" class="size-4 animate-spin" />
                            <Check v-else class="size-4" />
                            Salvar categoria
                        </Button>
                    </div>
                </div>
            </div>

            <Button v-if="vm.isAdmin" class="mt-4 self-start" :disabled="vm.salvando" @click="vm.salvar">
                <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                <Check v-else class="size-4" />
                Salvar
            </Button>
        </AppFieldset>

        <AppFieldset v-if="!vm.loading" legend="Quanto vender pra fechar o mês" class="mt-4">
            <template #legend>
                <span class="flex items-center gap-2">
                    <TrendingUp class="size-4" />
                    <span>Quanto vender pra fechar o mês</span>
                </span>
            </template>

            <div class="flex max-w-2xl flex-col gap-4">
                <!-- Custos fixos discriminados: com itens, o total é a soma (só leitura) -->
                <div>
                    <label class="mb-1 block text-sm font-medium">Custos fixos do mês, um a um</label>
                    <small class="mb-2 block text-muted-foreground">
                        Aluguel, salário (inclusive a sua retirada), energia, contador, DAS, juros de
                        fornecedor, depreciação de equipamento, reserva para reinvestir… Cadastrando aqui,
                        o total abaixo vira a soma automática e o painel de preços mostra a composição.
                    </small>

                    <AppDataTable
                        v-if="vm.custosFixos.length"
                        :rows="vm.custosFixos"
                        row-key="nome"
                        class="mb-3"
                        :page-size-options="[]"
                        :columns="[
                            { key: 'nome', label: 'Custo' },
                            { key: 'valor', label: 'Valor por mês' },
                            ...(vm.isAdmin ? [{ key: 'acoes', label: '', class: 'w-24' }] : []),
                        ]"
                    >
                        <template #cell-valor="{ row }">{{ formatCentavos(row.valor_centavos) }}</template>
                        <template #cell-acoes="{ row }">
                            <div class="flex gap-1">
                                <Button variant="ghost" size="icon-sm" @click="vm.editarCusto(row)">
                                    <Pencil class="size-4" />
                                </Button>
                                <Button variant="ghost" size="icon-sm" class="text-destructive" @click="vm.removerCusto(row.nome)">
                                    <Trash2 class="size-4" />
                                </Button>
                            </div>
                        </template>
                    </AppDataTable>

                    <div v-if="vm.isAdmin" class="flex flex-wrap items-end gap-3">
                        <div class="flex flex-col gap-1">
                            <label for="custo-nome" class="text-xs text-muted-foreground">Custo</label>
                            <Input id="custo-nome" v-model="vm.novoCusto.nome" placeholder="Ex.: Aluguel, DAS…" class="w-48" />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label for="custo-valor" class="text-xs text-muted-foreground">Valor por mês</label>
                            <InputMoney id="custo-valor" v-model="vm.novoCusto.valor" class="w-40" />
                        </div>
                        <Button
                            size="sm"
                            :disabled="!vm.novoCusto.nome || vm.novoCusto.valor == null || vm.salvandoCusto"
                            @click="vm.salvarCusto"
                        >
                            <LoaderCircle v-if="vm.salvandoCusto" class="size-4 animate-spin" />
                            <Check v-else class="size-4" />
                            Salvar custo
                        </Button>
                    </div>
                </div>

                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                    <div class="flex flex-col gap-2">
                        <label for="custos-fixos" class="text-sm font-medium">
                            Custos fixos por mês{{ vm.custosFixos.length ? ' (soma dos itens acima)' : '' }}
                        </label>
                        <InputMoney
                            id="custos-fixos"
                            v-model="vm.custosFixosMensais"
                            :disabled="!vm.isAdmin || vm.custosFixos.length > 0"
                        />
                        <small class="text-muted-foreground">
                            {{ vm.custosFixos.length ? 'Atualizado automaticamente ao editar os itens.' : 'Ou informe só o total, se preferir não detalhar.' }}
                        </small>
                    </div>
                    <div class="flex flex-col gap-2">
                        <label for="faturamento-mensal" class="text-sm font-medium">Faturamento esperado por mês</label>
                        <InputMoney
                            id="faturamento-mensal"
                            v-model="vm.faturamentoMensal"
                            :disabled="!vm.isAdmin"
                        />
                        <small class="text-muted-foreground">
                            Use o faturamento de <strong>hoje</strong>, não o desejado: ele divide os custos
                            fixos entre as vendas (cada uma contribui com a mesma fração do próprio valor).
                            Superestimar faz os preços carregarem custo fixo a menos — a sobra some sem
                            aparecer. Conforme a loja crescer, o sistema avisa quando atualizar.
                        </small>
                    </div>
                    <div class="flex flex-col gap-2">
                        <label for="meta-faturamento" class="text-sm font-medium">Meta de faturamento por mês</label>
                        <InputMoney
                            id="meta-faturamento"
                            v-model="vm.metaFaturamento"
                            :disabled="!vm.isAdmin"
                        />
                        <small class="text-muted-foreground">
                            Aqui sim entra a ambição: o dashboard mostra o progresso do mês rumo a esta meta.
                            Vender mais é o que engorda a sobra final — os custos fixos ficam mais "diluídos"
                            — sem precisar mexer nos preços. Não afeta a sugestão de preço.
                        </small>
                    </div>
                </div>

                <MessageBox v-if="vm.custoFixoPct > 0" severity="info">
                    Com esses números, <strong>{{ vm.custoFixoPct.toFixed(1) }}%</strong> de cada venda vai
                    para cobrir os custos fixos — um item de R$ 10 contribui com
                    {{ formatCentavos(Math.round(1000 * vm.custoFixoPct / 100)) }}, um de R$ 100 com
                    {{ formatCentavos(Math.round(10000 * vm.custoFixoPct / 100)) }}.
                </MessageBox>

                <MessageBox v-if="vm.breakEven" severity="info">
                    Você precisa vender cerca de <strong>{{ vm.breakEven.unidades }} unidades</strong>
                    ({{ formatCentavos(vm.breakEven.receitaCentavos) }}) por mês só pra cobrir os custos
                    fixos — considerando que cada venda deixa, em média,
                    {{ formatCentavos(vm.breakEven.margemContribuicaoMediaCentavos) }}.
                </MessageBox>
            </div>

            <Button v-if="vm.isAdmin" class="mt-4 self-start" :disabled="vm.salvando" @click="vm.salvar">
                <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                <Check v-else class="size-4" />
                Salvar
            </Button>
        </AppFieldset>
    </div>
</template>
