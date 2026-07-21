import type { Opcao } from "~/models/shared";
import type { Produto } from "~/models/catalogo";
import { listarProdutos } from "~/models/catalogo";
import type { Cliente } from "~/models/crm";
import { listarClientes } from "~/models/crm";
import type { Saldo } from "~/models/estoque";
import { listarSaldos } from "~/models/estoque";
import type {
  Orcamento,
  OrcamentoDetalhes,
  OrcamentoItem,
} from "~/models/orcamentos";
import {
  aceitarOrcamento,
  adicionarItem,
  aplicarDesconto,
  atualizarCabecalho,
  buscarOrcamento,
  cancelarOrcamento,
  criarOrcamento,
  emitirOrcamento,
  listarOrcamentos,
  recusarOrcamento,
  removerItem,
} from "~/models/orcamentos";

/** ViewModel da página de Orçamentos: concentra estado e regras de negócio;
 * a View (página + dialogs) só lê estado e dispara ações. */
export function useOrcamentosViewModel() {
  const { apiFetch, apiErrorMessage } = useApi();
  const { toCentavos } = useFormat();
  const { notifySuccess, notifyError } = useNotify();

  // --- Listagem ---
  const orcamentos = ref<Orcamento[]>([]);
  const produtos = ref<Produto[]>([]);
  const clientes = ref<Cliente[]>([]);
  const saldos = ref<Saldo[]>([]);
  const loading = ref(false);
  const filtro = ref("");

  const saldoPorProduto = computed(() => {
    const m = new Map<string, number>();
    saldos.value.forEach((s) => m.set(s.produto_id, s.quantidade));
    return m;
  });

  const clientePorId = computed(() => {
    const m = new Map<string, string>();
    clientes.value.forEach((c) => m.set(c.cliente_id, c.nome));
    return m;
  });

  /** Único ponto que decide o que exibir para "cliente": cadastro do CRM
   * (cliente_id) tem prioridade, senão o nome avulso digitado no balcão,
   * senão "Sem cliente". Usada tanto na listagem quanto nos dialogs — não
   * duplicar esta regra em outro lugar. */
  function nomeCliente(o: { cliente_id: string | null; cliente_avulso: string | null }) {
    if (o.cliente_id) return clientePorId.value.get(o.cliente_id) ?? "—";
    if (o.cliente_avulso) return o.cliente_avulso;
    return "Sem cliente";
  }

  /** Nome + telefone/documento numa linha (cadastrado), ou "nome (avulso)"
   * para cliente de balcão — para uso em impressões. */
  function clienteResumo(o: { cliente_id: string | null; cliente_avulso: string | null }): string {
    if (!o.cliente_id) {
      return o.cliente_avulso ? `${o.cliente_avulso} (avulso)` : "Sem cliente";
    }
    const c = clientes.value.find((x) => x.cliente_id === o.cliente_id);
    if (!c) return "—";
    const contato = c.telefone || c.cpf_cnpj;
    return contato ? `${c.nome} (${contato})` : c.nome;
  }

  function podeExcluir(orcamento: Orcamento) {
    return orcamento.status === "Rascunho" || orcamento.status === "Emitido";
  }

  const { buscarAproximado } = useFuzzySearch();

  const orcamentosFiltrados = computed(() =>
    buscarAproximado(orcamentos.value, filtro.value, (o) => `${nomeCliente(o)} ${o.status}`),
  );

  /** Mostra o saldo em estoque no rótulo — o vendedor precisa ter ciência da
   * disponibilidade antes de orçar (mesmo quando o tenant permite orçar sem
   * estoque, a informação continua relevante para a conversa com o cliente). */
  const opcoesProduto = computed<Opcao[]>(() =>
    produtos.value
      .filter((p) => p.ativo)
      .map((p) => ({
        label: p.controla_estoque
          ? `${p.sku} — ${p.descricao} · ${saldoPorProduto.value.get(p.produto_id) ?? 0} em estoque`
          : `${p.sku} — ${p.descricao} · serviço`,
        value: p.produto_id,
      })),
  );

  const opcoesCliente = computed<Opcao[]>(() =>
    clientes.value.map((c) => ({ label: c.nome, value: c.cliente_id })),
  );

  async function carregar() {
    loading.value = true;
    try {
      const [{ orcamentos: o }, { produtos: p }, { clientes: c }, { saldos: s }] =
        await Promise.all([
          listarOrcamentos(apiFetch),
          listarProdutos(apiFetch),
          listarClientes(apiFetch),
          listarSaldos(apiFetch),
        ]);
      orcamentos.value = o;
      produtos.value = p;
      clientes.value = c;
      saldos.value = s;
    } catch (e) {
      notifyError(apiErrorMessage(e));
    } finally {
      loading.value = false;
    }
  }

  // --- Novo orçamento ---
  const novoVisible = ref(false);

  async function criar(payload: {
    cliente_id: string | null;
    cliente_avulso: string | null;
    validade_dias: number;
  }) {
    try {
      const { orcamento_id } = await criarOrcamento(apiFetch, payload);
      novoVisible.value = false;
      await carregar();
      await abrirDetalhe(orcamento_id);
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  // --- Detalhe ---
  const detalheVisible = ref(false);
  const detalhe = ref<OrcamentoDetalhes | null>(null);

  const { printReceipt } = useThermalPrint();
  const { tenantSlug } = useAuth();
  const { businessInfo, garantirCarregado: garantirEmpresaCarregada } = useEmpresaInfo();
  void garantirEmpresaCarregada();

  function imprimirOrcamento() {
    const d = detalhe.value;
    if (!d) return;
    // total_centavos já é líquido (itens − desconto); subtotal é a soma bruta.
    const subtotal = d.orcamento.total_centavos + d.orcamento.desconto_centavos;
    printReceipt({
      storeName: tenantSlug.value || "Finledger",
      title: "ORÇAMENTO",
      reference: `Nº ${d.orcamento.orcamento_id.slice(0, 8)} · ${d.orcamento.status}`,
      meta: [
        { label: "Cliente", value: clienteResumo(d.orcamento) },
        { label: "Validade", value: `${d.orcamento.validade_dias} dias` },
      ],
      businessInfo: businessInfo.value,
      items: d.itens.map((i) => ({
        descricao: i.descricao,
        sku: i.sku,
        quantidade: i.quantidade,
        unitCents: i.preco_unitario_centavos,
      })),
      subtotalCents: subtotal,
      discountCents: d.orcamento.desconto_centavos,
      totalCents: d.orcamento.total_centavos,
      footerNote: "Orçamento sem valor fiscal. Sujeito a disponibilidade de estoque.",
    });
  }
  const carregandoDetalhe = ref(false);

  const isRascunho = computed(
    () => detalhe.value?.orcamento.status === "Rascunho",
  );
  const isEmitido = computed(
    () => detalhe.value?.orcamento.status === "Emitido",
  );

  const subtotalCentavos = computed(
    () =>
      detalhe.value?.itens.reduce(
        (acc, item) => acc + item.preco_unitario_centavos * item.quantidade,
        0,
      ) ?? 0,
  );

  async function buscarDetalheAtual(id: string) {
    try {
      detalhe.value = await buscarOrcamento(apiFetch, id);
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  async function abrirDetalhe(id: string) {
    detalheVisible.value = true;
    carregandoDetalhe.value = true;
    await buscarDetalheAtual(id);
    carregandoDetalhe.value = false;
  }

  async function recarregar() {
    if (detalhe.value)
      await buscarDetalheAtual(detalhe.value.orcamento.orcamento_id);
    await carregar();
  }

  // --- Edição de cabeçalho (cliente / validade) ---
  const editCliente = ref<string | null>(null);
  const editClienteAvulso = ref("");
  const editValidade = ref(15);
  const salvandoEdicao = ref(false);

  watch(detalhe, (d) => {
    editCliente.value = d?.orcamento.cliente_id ?? null;
    editClienteAvulso.value = d?.orcamento.cliente_avulso ?? "";
    editValidade.value = d?.orcamento.validade_dias ?? 15;
  });

  // Mesma exclusividade mútua do dialog de criação.
  watch(editCliente, (v) => {
    if (v) editClienteAvulso.value = "";
  });

  async function salvarEdicao() {
    if (!detalhe.value) return;
    salvandoEdicao.value = true;
    try {
      await atualizarCabecalho(apiFetch, detalhe.value.orcamento.orcamento_id, {
        cliente_id: editCliente.value,
        cliente_avulso: editClienteAvulso.value.trim() || null,
        validade_dias: editValidade.value,
      });
      notifySuccess("Orçamento atualizado", undefined, 2500);
      await recarregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    } finally {
      salvandoEdicao.value = false;
    }
  }

  // --- Itens ---
  const novoItem = reactive({
    produto_id: null as string | null,
    quantidade: 1,
  });

  async function adicionarItemAtual() {
    if (!detalhe.value || !novoItem.produto_id) return;
    const p = produtos.value.find((x) => x.produto_id === novoItem.produto_id);
    if (!p) return;
    try {
      await adicionarItem(apiFetch, detalhe.value.orcamento.orcamento_id, {
        produto_id: p.produto_id,
        sku: p.sku,
        descricao: p.descricao,
        quantidade: novoItem.quantidade,
        preco_unitario_centavos: p.preco_venda,
      });
      novoItem.produto_id = null;
      novoItem.quantidade = 1;
      await recarregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  async function removerItemAtual(item: OrcamentoItem) {
    if (!detalhe.value) return;
    try {
      await removerItem(
        apiFetch,
        detalhe.value.orcamento.orcamento_id,
        item.item_id,
      );
      await recarregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  // --- Desconto ---
  const descontoValor = ref(0);

  /** Sincroniza o valor digitado a cada tecla (o InputNumber currency só
   * atualiza o v-model no blur — clicar direto em "Aplicar" enviava 0). */
  function sincronizarDesconto(valor: number | string | null | undefined) {
    descontoValor.value = typeof valor === "number" ? valor : Number(valor) || 0;
  }

  async function aplicarDescontoAtual() {
    if (!detalhe.value) return;
    try {
      await aplicarDesconto(
        apiFetch,
        detalhe.value.orcamento.orcamento_id,
        toCentavos(descontoValor.value),
      );
      notifySuccess("OK", "Desconto aplicado.", 2500);
      await recarregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  // --- Transições de status ---
  async function emitir() {
    if (!detalhe.value) return;
    try {
      await emitirOrcamento(apiFetch, detalhe.value.orcamento.orcamento_id);
      await recarregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  async function aceitar() {
    if (!detalhe.value) return;
    try {
      await aceitarOrcamento(apiFetch, detalhe.value.orcamento.orcamento_id);
      notifySuccess(
        "Orçamento aceito",
        "Uma venda foi gerada — recupere-a no PDV para finalizar o pagamento.",
      );
      await recarregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  const recusarVisible = ref(false);

  async function onRecusar(motivo: string) {
    if (!detalhe.value) return;
    try {
      await recusarOrcamento(
        apiFetch,
        detalhe.value.orcamento.orcamento_id,
        motivo,
      );
      recusarVisible.value = false;
      await recarregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  // --- Exclusão (cancelamento) ---
  const excluirVisible = ref(false);
  const orcamentoParaExcluir = ref<Orcamento | null>(null);

  function abrirExclusao(orcamento: Orcamento) {
    orcamentoParaExcluir.value = orcamento;
    excluirVisible.value = true;
  }

  async function onExcluir(motivo: string) {
    const alvo = orcamentoParaExcluir.value ?? detalhe.value?.orcamento;
    if (!alvo) return;
    try {
      await cancelarOrcamento(apiFetch, alvo.orcamento_id, motivo);
      notifySuccess("Orçamento excluído", "Orçamento cancelado com sucesso.");
      excluirVisible.value = false;
      orcamentoParaExcluir.value = null;
      if (detalhe.value?.orcamento.orcamento_id === alvo.orcamento_id)
        await recarregar();
      else await carregar();
    } catch (e) {
      notifyError(apiErrorMessage(e));
    }
  }

  return reactive({
    // listagem
    produtos,
    clientes,
    loading,
    filtro,
    orcamentosFiltrados,
    nomeCliente,
    podeExcluir,
    opcoesProduto,
    opcoesCliente,
    carregar,
    // novo
    novoVisible,
    criar,
    // detalhe
    detalheVisible,
    detalhe,
    imprimirOrcamento,
    carregandoDetalhe,
    isRascunho,
    isEmitido,
    subtotalCentavos,
    abrirDetalhe,
    recarregar,
    // edição de cabeçalho
    editCliente,
    editClienteAvulso,
    editValidade,
    salvandoEdicao,
    salvarEdicao,
    // itens
    novoItem,
    adicionarItem: adicionarItemAtual,
    removerItem: removerItemAtual,
    // desconto
    descontoValor,
    sincronizarDesconto,
    aplicarDesconto: aplicarDescontoAtual,
    // transições
    emitir,
    aceitar,
    recusarVisible,
    onRecusar,
    excluirVisible,
    orcamentoParaExcluir,
    abrirExclusao,
    onExcluir,
  });
}
