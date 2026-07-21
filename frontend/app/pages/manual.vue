<script setup lang="ts">
/** Manual do usuário: o que cada módulo faz, como usar e — principalmente —
 * como os módulos conversam entre si (uma ação aqui muda comportamento ali).
 * Conteúdo estático; screenshots em /public/manual. */
import {
    ArrowRight,
    BarChart3,
    Box,
    FileCheck2,
    FileText,
    GitBranch,
    Home,
    Lightbulb,
    Monitor,
    Percent,
    RotateCcw,
    Settings,
    ShoppingCart,
    Tag,
    Truck,
    Users,
    Wallet,
    Zap,
    type LucideIcon,
} from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'

interface SecaoModulo {
    id: string
    titulo: string
    icon: LucideIcon
    rota: string
    img: string
    resumo: string
    pontos: string[]
    conexoes?: string
}

const modulos: SecaoModulo[] = [
    {
        id: 'dashboard',
        titulo: 'Dashboard',
        icon: Home,
        rota: '/',
        img: '/manual/dashboard.png',
        resumo:
            'A primeira tela do dia: saúde do negócio em linguagem simples e a lista "O que fazer hoje" com as recomendações mais importantes do sistema.',
        pontos: [
            'O anel "Saúde do negócio" resume tudo numa nota de 0 a 100, composta por caixa, cobrança, margem, giro de estoque, tendência de vendas e o rumo à meta — cada componente mostra a própria nota e o motivo. Verde (80+) é saudável; abaixo de 60 há ponto crítico.',
            'A barra "Meta do mês" mostra quanto já foi vendido rumo à meta de faturamento e o ritmo necessário por dia útil para fechá-la — vender mais é o que engorda a sobra final (veja "Entendendo as margens").',
            'Os 4 cartões de cima mostram vendas do mês, dinheiro esperado em 30 dias, atrasos de clientes e a margem de balcão média (preço − custo do produto; dela ainda saem custos fixos e taxas) — verde é bom, vermelho pede atenção.',
            '"O que fazer hoje" traz no máximo 3 recomendações por vez; o sino 🔔 no topo guarda a lista completa. Em cada uma você pode Abrir, marcar Resolvido ou Ignorar por 30 dias.',
            'Ignorar um tipo de alerta repetidamente ensina o sistema a dar menos peso a ele — o motor aprende com o seu feedback.',
        ],
        conexoes:
            'Tudo aqui é calculado a partir dos outros módulos: vendas, financeiro e estoque alimentam os indicadores e os alertas.',
    },
    {
        id: 'terminal',
        titulo: 'Terminal PDV',
        icon: Monitor,
        rota: '/terminal',
        img: '/manual/terminal.png',
        resumo:
            'O balcão da loja: registrar uma venda rápida, escolhendo produtos, quantidade e forma de pagamento em tela cheia.',
        pontos: [
            'Busque o produto por código ou descrição; o sistema mostra o saldo em estoque na hora.',
            'Produto sem saldo pode ser vendido "sob encomenda", confirmando item a item.',
            'Ao confirmar a venda, tudo acontece sozinho: estoque baixa, conta a receber é criada e a nota fiscal é emitida.',
        ],
        conexoes:
            'Confirmar venda → baixa o Estoque → cria a conta no Financeiro → emite a NF no Fiscal. Uma única ação, quatro módulos.',
    },
    {
        id: 'vendas',
        titulo: 'Vendas',
        icon: ShoppingCart,
        rota: '/vendas',
        img: '/manual/vendas.png',
        resumo: 'Histórico de todas as vendas, com detalhe de itens, impressão e devoluções.',
        pontos: [
            'Vendas "iniciadas" ainda não movimentaram nada — só a confirmação dispara estoque, financeiro e fiscal.',
            'Devolução parcial: os itens voltam ao estoque pelo custo médio atual.',
            'Devolução total: além do estoque, a venda é cancelada, a conta a receber em aberto é estornada e a nota fiscal entra em cancelamento.',
            'Limpeza automática: vendas abandonadas ou canceladas somem da listagem após o prazo definido em Configurações — nada é excluído; o administrador vê e restaura tudo pela Lixeira (botão no topo da tela).',
        ],
        conexoes:
            'A devolução é o caminho inverso da venda: mexe em Estoque, Financeiro e Fiscal na mesma proporção do que foi devolvido.',
    },
    {
        id: 'orcamentos',
        titulo: 'Orçamentos',
        icon: FileText,
        rota: '/orcamentos',
        img: '/manual/orcamentos.png',
        resumo:
            'Proposta antes da venda: monte o orçamento, imprima/envie ao cliente e acompanhe até virar venda (ou expirar).',
        pontos: [
            'Fluxo: rascunho → emitido → aceito → convertido em venda. Recusas e expirações também ficam registradas.',
            'Cada orçamento tem validade em dias; o sistema avisa quando um está prestes a vencer sem resposta.',
            'Em Configurações você decide se o vendedor pode orçar produto sem saldo em estoque.',
            'Limpeza automática: orçamentos que não viraram venda (rascunhos antigos, recusados, expirados, cancelados) vão para a Lixeira após o prazo configurado — restauráveis a qualquer momento pelo administrador.',
        ],
        conexoes:
            'Converter um orçamento cria a venda com os mesmos itens — e daí segue o fluxo normal de venda. A taxa de conversão alimenta as Análises.',
    },
    {
        id: 'clientes',
        titulo: 'Clientes',
        icon: Users,
        rota: '/clientes',
        img: '/manual/clientes.png',
        resumo: 'Cadastro de clientes com histórico de compras e controle de crédito.',
        pontos: [
            'Peça o CPF/CNPJ no caixa: vendas identificadas alimentam as análises de clientes (quem compra sempre, quem sumiu).',
            'Cliente com atraso longo pode ser bloqueado para novas vendas a prazo — o sistema até sugere isso via alerta.',
        ],
        conexoes:
            'O saldo devedor vem do Financeiro; a importância de cada cliente (campeão, em risco, perdido) vem das vendas e aparece nas Análises.',
    },
    {
        id: 'catalogo',
        titulo: 'Catálogo',
        icon: Tag,
        rota: '/catalogo',
        img: '/manual/catalogo.png',
        resumo:
            'Todos os produtos e serviços da loja: descrição, NCM, categoria, preços de custo e venda — e o assistente de precificação.',
        pontos: [
            'Ao cadastrar ou editar preços, o painel de sugestão mostra o preço recomendado e o passo a passo completo da conta ("Como chegamos neste preço?").',
            'A sugestão considera: custo do produto (usando o custo médio real do estoque quando ele for maior que o do cadastro), custos fixos como fração do preço (rateio proporcional), imposto/comissão/frete, a taxa de cartão ponderada pelo uso real e a margem desejada com ajuste de giro (produto parado ganha desconto até lucro zero; produto que vende bem sustenta margem cheia).',
            'Margens podem ser configuradas por loja, por categoria ou por produto — a mais específica vence.',
            'Registre preços vistos na concorrência para comparar na hora de decidir.',
        ],
        conexoes:
            'O custo vem das entradas de Estoque/Compras; as regras vêm de Configurações; o giro vem das Vendas. Nada é aplicado sozinho — você sempre decide o preço final.',
    },
    {
        id: 'estoque',
        titulo: 'Estoque',
        icon: Box,
        rota: '/estoque',
        img: '/manual/estoque.png',
        resumo: 'Saldo físico de cada produto, custo médio, entradas, ajustes e estoque mínimo.',
        pontos: [
            'Toda entrada registra quantidade E custo unitário — é assim que o custo médio (e o valor total do estoque) fica correto.',
            'Ajustes de inventário exigem justificativa e ficam registrados.',
            'Defina o estoque mínimo por produto: ao atingi-lo, o sistema gera alerta e sugere pedido de compra com 1 clique.',
        ],
        conexoes:
            'Vendas baixam o saldo; devoluções e compras recebidas reentram; o custo médio alimenta o valor do estoque, a margem real das Análises e a sugestão de preço.',
    },
    {
        id: 'fornecedores',
        titulo: 'Fornecedores e Compras',
        icon: Truck,
        rota: '/compras',
        img: '/manual/compras.png',
        resumo:
            'Cadastro de fornecedores e o ciclo de reposição: pedido de compra → aprovação → recebimento.',
        pontos: [
            'O pedido nasce manualmente ou pré-preenchido a partir de um alerta de ruptura.',
            'Ao receber o pedido, a entrada no estoque acontece com o custo da compra e a conta a pagar é criada no Financeiro.',
            'Pedidos aprovados e esquecidos (sem envio ao fornecedor) geram aviso nas Análises.',
        ],
        conexoes:
            'Receber compra → entrada em Estoque (recalcula custo médio) → conta a pagar no Financeiro. O frete típico do fornecedor pré-preenche o custo da entrada.',
    },
    {
        id: 'financeiro',
        titulo: 'Financeiro',
        icon: Wallet,
        rota: '/financeiro',
        img: '/manual/financeiro.png',
        resumo: 'Contas a receber e a pagar: o que entra, o que sai, o que está vencido.',
        pontos: [
            'Contas a receber nascem das vendas (a prazo); contas a pagar, das compras recebidas — você também pode lançar avulsas.',
            'Registre recebimentos e pagamentos (baixas parciais ou totais).',
            'O filtro "vencidas" é a sua lista de cobrança do dia.',
        ],
        conexoes:
            'É a ponta do fluxo de vendas e compras. Alimenta os indicadores de caixa do Dashboard e os alertas ("vai faltar dinheiro em 30 dias", "cliente devendo sem bloqueio").',
    },
    {
        id: 'fiscal',
        titulo: 'Fiscal',
        icon: FileCheck2,
        rota: '/fiscal',
        img: '/manual/fiscal.png',
        resumo: 'Notas fiscais emitidas a partir das vendas, com status de autorização.',
        pontos: [
            'Cada venda confirmada gera sua NF automaticamente.',
            'Notas rejeitadas há mais de 24h geram alerta — corrija e reenvie.',
            'Devolução total marca a NF para cancelamento (pendente enquanto a integração com a SEFAZ não estiver ativa).',
        ],
        conexoes: 'Espelha o módulo de Vendas: cada venda tem sua nota; devoluções mexem na nota correspondente.',
    },
    {
        id: 'analises',
        titulo: 'Análises (BI)',
        icon: BarChart3,
        rota: '/analises',
        img: '/manual/analises.png',
        resumo:
            'O consultor do sistema: quatro abas que respondem "como está o dinheiro, as vendas, o estoque e os preços" — sempre com a ação recomendada.',
        pontos: [
            'Dinheiro e Caixa: ciclo de caixa, projeção de entradas/saídas por 12 semanas e a lista de quem cobrar primeiro.',
            'Vendas e Clientes: funil de orçamentos, desempenho por vendedor e clientes importantes que sumiram.',
            'Estoque e Compras: o que merece atenção, o que pode faltar, o que está parado prendendo dinheiro.',
            'Preços e Margens: compara o preço praticado com o sugerido produto a produto, mostra a margem real vs a meta e o ganho potencial — cada linha tem o "Por quê?" com o passo a passo do cálculo.',
        ],
        conexoes:
            'Lê todos os módulos e devolve recomendações. Os alertas daqui aparecem no sino e no "O que fazer hoje" do Dashboard, atualizados a cada 5 minutos.',
    },
    {
        id: 'configuracoes',
        titulo: 'Configurações',
        icon: Settings,
        rota: '/configuracoes',
        img: '/manual/configuracoes.png',
        resumo:
            'As regras do seu negócio: dados da empresa, comportamento do estoque em orçamentos e toda a precificação assistida.',
        pontos: [
            'Dados da empresa saem nas impressões de venda e orçamento.',
            'Precificação: margem desejada, percentuais que saem do preço (imposto, comissão, frete), máquinas de cartão (a sugestão usa a taxa mais cara, por segurança) e exceções por categoria.',
            'Custos fixos podem ser detalhados um a um (aluguel, salário, DAS, combustível…) — o total vira a soma automática e a composição aparece no passo a passo do preço.',
            'Informe o faturamento esperado por mês (o de HOJE, não o desejado): os custos fixos viram um percentual do preço de cada venda. O sistema avisa quando o número real divergir e for hora de atualizar.',
            'Defina também a meta de faturamento — o alvo de crescimento acompanhado no Dashboard. Meta e faturamento esperado são coisas diferentes: um mede a ambição, o outro calibra os preços. Antes de mexer nas margens, leia "Entendendo as margens" neste manual.',
            'Limpeza automática (lixeira): defina em quantos dias vendas e orçamentos não concretizados saem das listagens. Nada é excluído — ficam na Lixeira de cada tela, restauráveis pelo administrador.',
        ],
        conexoes:
            'Tudo configurado aqui muda o comportamento do assistente de preços no Catálogo, no Estoque e na aba Preços e Margens das Análises — na hora.',
    },
    {
        id: 'usuarios',
        titulo: 'Usuários',
        icon: Users,
        rota: '/usuarios',
        img: '/manual/usuarios.png',
        resumo: 'Quem acessa o sistema e com qual papel.',
        pontos: [
            'Administradores configuram o sistema e veem tudo; vendedores operam o dia a dia.',
            'Cada venda fica associada ao vendedor logado — é isso que alimenta o desempenho por vendedor nas Análises.',
        ],
    },
]

const fluxos = [
    {
        icon: ShoppingCart,
        titulo: 'Venda confirmada',
        passos: ['Estoque baixa na hora', 'Conta a receber criada no Financeiro', 'Nota fiscal emitida no Fiscal'],
    },
    {
        icon: RotateCcw,
        titulo: 'Devolução',
        passos: [
            'Itens voltam ao Estoque pelo custo médio',
            'Se for total: venda cancelada + conta estornada',
            'Nota fiscal marcada para cancelamento',
        ],
    },
    {
        icon: FileText,
        titulo: 'Orçamento aceito',
        passos: ['Convertido vira Venda com os mesmos itens', 'Segue o fluxo normal de venda', 'Conversão medida nas Análises'],
    },
    {
        icon: Truck,
        titulo: 'Compra recebida',
        passos: ['Entrada no Estoque com o custo da compra', 'Custo médio recalculado', 'Conta a pagar criada no Financeiro'],
    },
    {
        icon: Tag,
        titulo: 'Sugestão de preço',
        passos: [
            'Custo (Estoque/Compras) + custos fixos proporcionais ao preço (Configurações)',
            'Percentuais e margem (Configurações)',
            'Giro do produto (Vendas) ajusta a margem',
        ],
    },
    {
        icon: Zap,
        titulo: 'Motor de alertas',
        passos: [
            'Lê todos os módulos a cada 5 minutos',
            'Prioriza por impacto em R$ e urgência',
            'Entrega no sino 🔔 e no "O que fazer hoje"',
        ],
    },
    {
        icon: RotateCcw,
        titulo: 'Limpeza automática',
        passos: [
            'Vendas/orçamentos não concretizados são arquivados após o prazo configurado',
            'Nada é excluído — vão para a Lixeira de cada tela',
            'O administrador restaura com um clique',
        ],
    },
]
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-6">
            <h1 class="text-2xl font-semibold">Manual do sistema</h1>
            <p class="text-muted-foreground">
                O que cada módulo faz, como usar e como uma ação num módulo afeta os outros.
            </p>
        </div>

        <!-- Navegação rápida -->
        <div class="mb-6 flex flex-wrap gap-2">
            <a v-for="m in modulos" :key="m.id" :href="`#${m.id}`" class="manual-chip">
                <component :is="m.icon" class="size-3.5" /> {{ m.titulo }}
            </a>
            <a href="#margens" class="manual-chip manual-chip-destaque"><Percent class="size-3.5" /> Entendendo as margens</a>
            <a href="#fluxos" class="manual-chip manual-chip-destaque"><GitBranch class="size-3.5" /> Como tudo se conecta</a>
        </div>

        <!-- Visão geral do fluxo -->
        <MessageBox severity="info" class="mb-6">
            O Finledger funciona em cadeia: <strong>Orçamento → Venda → Estoque → Financeiro → Fiscal</strong>,
            com o BI lendo tudo e devolvendo recomendações. Você registra o fato (vendeu, comprou, recebeu) e o
            sistema propaga as consequências — nada precisa ser lançado duas vezes.
        </MessageBox>

        <!-- Módulos -->
        <section v-for="m in modulos" :id="m.id" :key="m.id" class="manual-secao">
            <div class="mb-1 flex items-center gap-2">
                <component :is="m.icon" class="size-5 text-primary" />
                <h2 class="m-0 text-xl font-semibold">{{ m.titulo }}</h2>
                <NuxtLink :to="m.rota" class="ml-auto">
                    <Button variant="ghost" size="sm">
                        Abrir módulo
                        <ArrowRight class="size-4" />
                    </Button>
                </NuxtLink>
            </div>
            <p class="mb-3 text-muted-foreground">{{ m.resumo }}</p>

            <img :src="m.img" :alt="`Tela de ${m.titulo}`" class="manual-print" loading="lazy" >

            <ul class="manual-pontos">
                <li v-for="(p, i) in m.pontos" :key="i">{{ p }}</li>
            </ul>

            <p v-if="m.conexoes" class="manual-conexao">
                <GitBranch class="mr-1 inline size-3.5" /><strong>Conexões:</strong> {{ m.conexoes }}
            </p>
        </section>

        <!-- Entendendo as margens (sem contabilês) -->
        <section id="margens" class="manual-secao">
            <div class="mb-1 flex items-center gap-2">
                <Percent class="size-5 text-primary" />
                <h2 class="m-0 text-xl font-semibold">Entendendo as margens — sem contabilês</h2>
            </div>
            <p class="mb-4 text-muted-foreground">
                Existem <strong>duas margens diferentes</strong>, e confundi-las é o erro mais comum ao
                configurar o sistema. Leia isto antes de mexer em "Preço de venda" nas Configurações.
            </p>

            <div class="mb-4 grid grid-cols-1 gap-4 md:grid-cols-2">
                <Card>
                    <CardContent>
                        <p class="mb-1 font-semibold">Margem "de balcão" (bruta)</p>
                        <p class="mb-0 text-sm text-muted-foreground">
                            A diferença entre o preço e o que você pagou pelo produto. É o número que todo
                            comerciante tem na cabeça — "compro por 10, vendo por 17, ganho 40%". Só que
                            desse dinheiro ainda saem o aluguel, o salário, o cartão… Ele <strong>não é
                            lucro</strong>: é o que a venda arrecada para pagar as contas.
                        </p>
                    </CardContent>
                </Card>
                <Card>
                    <CardContent>
                        <p class="mb-1 font-semibold">Sobra final (líquida) — a que o sistema usa</p>
                        <p class="mb-0 text-sm text-muted-foreground">
                            O que fica <strong>depois de pagar tudo</strong>: o produto, a fatia dos custos
                            fixos e as taxas. É o campo "quanto você quer que sobre" nas Configurações.
                            Números pequenos aqui são normais e saudáveis — ainda mais quando o seu próprio
                            salário já está nos custos fixos: a sobra é lucro <em>além</em> da sua retirada.
                        </p>
                    </CardContent>
                </Card>
            </div>

            <p class="mb-2 text-sm font-medium">Exemplo com números redondos (loja com R$ 1.900 de custos fixos e cartão de 3,5%):</p>
            <ul class="manual-pontos mb-4">
                <li>Você compra um produto por <strong>R$ 60</strong> e vende por <strong>R$ 100</strong> → margem de balcão de <strong>40%</strong> (R$ 40).</li>
                <li>Desses R$ 40: ~R$ 32 pagam a fatia dos custos fixos (se a loja fatura R$ 6.000/mês, cada venda contribui com ~32% do próprio valor) e R$ 3,50 vão para o cartão.</li>
                <li>Sobra final: <strong>~R$ 5 (5%)</strong>. O produto não "deu 40% de lucro" — ele arrecadou 40% para pagar as contas e sobrou 5%.</li>
            </ul>

            <p class="mb-2 text-sm font-medium">E é por isso que vender mais vale mais que aumentar preço — os custos fixos não crescem junto:</p>
            <div class="mb-3 overflow-x-auto">
                <table class="manual-tabela">
                    <thead>
                        <tr><th>Faturamento/mês</th><th>Fatia dos fixos em cada venda</th><th>Sobra final (mesmos preços)</th></tr>
                    </thead>
                    <tbody>
                        <tr><td>R$ 6.000</td><td>~32%</td><td>~5%</td></tr>
                        <tr><td>R$ 8.000</td><td>~24%</td><td>~13%</td></tr>
                        <tr><td>R$ 10.000</td><td>~19%</td><td>~17%</td></tr>
                    </tbody>
                </table>
            </div>

            <p class="mb-2 text-sm font-medium">Três comportamentos do sistema que parecem estranhos — mas são fundamentos:</p>
            <ul class="manual-pontos mb-4">
                <li>
                    <strong>Produto parado pode ter sugestão com lucro zero.</strong> Se um produto está meses
                    sem vender, o sistema reduz a margem dele até, no limite, zerar o lucro — o preço ainda
                    paga o produto, as taxas e a fatia dos custos fixos. Dinheiro parado na prateleira não
                    paga conta; virar estoque em caixa vale mais que insistir numa margem que não acontece.
                </li>
                <li>
                    <strong>A taxa do cartão pesa conforme o uso real.</strong> Se metade das suas vendas é
                    em Pix ou dinheiro, o cartão só "morde" metade do que morderia — com histórico
                    suficiente (20+ vendas), o sistema pondera a taxa pela participação real do cartão. Sem
                    histórico, assume o pior caso (tudo no cartão), por segurança.
                </li>
                <li>
                    <strong>A sugestão vira preço "de prateleira".</strong> A conta exata (ex.: R$ 50,76) é
                    arredondada <em>para cima</em> ao próximo final comercial (R$ 50,90) — nunca para baixo,
                    para o arredondamento não comer a margem. O passo a passo mostra os dois números.
                </li>
            </ul>

            <p class="manual-conexao">
                <Lightbulb class="mr-1 inline size-3.5" /><strong>Regra de bolso:</strong> mantenha a margem de
                balcão competitiva, defina uma sobra final honesta (5–15%) nas Configurações e persiga a
                <strong>meta de faturamento</strong> no Dashboard — o crescimento do volume é o que engorda
                a sobra, e o sistema avisa quando o rateio dos custos fixos precisar ser atualizado.
            </p>
        </section>

        <!-- Como tudo se conecta -->
        <section id="fluxos" class="manual-secao">
            <div class="mb-1 flex items-center gap-2">
                <GitBranch class="size-5 text-primary" />
                <h2 class="m-0 text-xl font-semibold">Como tudo se conecta</h2>
            </div>
            <p class="mb-4 text-muted-foreground">
                Os encadeamentos automáticos do sistema — uma ação sua dispara tudo isto sem lançamento manual.
            </p>
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
                <Card v-for="f in fluxos" :key="f.titulo">
                    <CardContent>
                        <div class="mb-2 flex items-center gap-2">
                            <component :is="f.icon" class="size-4 text-primary" />
                            <span class="font-semibold">{{ f.titulo }}</span>
                        </div>
                        <ol class="manual-fluxo">
                            <li v-for="(p, i) in f.passos" :key="i">{{ p }}</li>
                        </ol>
                    </CardContent>
                </Card>
            </div>
        </section>
    </div>
</template>

<style scoped>
.manual-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 9999px;
    font-size: 0.8125rem;
    color: var(--foreground);
    transition: border-color 0.15s, color 0.15s;
}

.manual-chip:hover {
    border-color: var(--primary);
    color: var(--primary);
}

.manual-chip-destaque {
    border-color: var(--primary);
    color: var(--primary);
    font-weight: 600;
}

.manual-secao {
    padding: 1.5rem 0;
    scroll-margin-top: 5rem;
}

.manual-secao + .manual-secao {
    border-top: 1px solid var(--border);
}

.manual-print {
    width: 100%;
    max-width: 56rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 2px 8px rgb(0 0 0 / 0.06);
    margin-bottom: 1rem;
}

.manual-pontos {
    margin: 0 0 0.75rem;
    padding-left: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    font-size: 0.9rem;
}

.manual-conexao {
    margin: 0;
    padding: 0.625rem 0.875rem;
    border-left: 3px solid var(--primary);
    border-radius: 4px;
    background: var(--muted);
    font-size: 0.875rem;
    color: var(--muted-foreground);
}

.manual-tabela {
    border-collapse: collapse;
    font-size: 0.875rem;
}

.manual-tabela th,
.manual-tabela td {
    border: 1px solid var(--border);
    padding: 0.375rem 0.875rem;
    text-align: left;
}

.manual-tabela th {
    background: var(--muted);
    font-weight: 600;
}

.manual-fluxo {
    margin: 0;
    padding-left: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.85rem;
    color: var(--muted-foreground);
}
</style>
