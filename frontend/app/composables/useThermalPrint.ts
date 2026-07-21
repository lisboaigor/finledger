/** Motor de impressão do sistema: dois formatos.
 *
 * - Cupons (80mm térmico): recibo de venda, orçamento, cupom do PDV —
 *   documentos rápidos, entregues ao cliente no balcão.
 * - Relatórios (A4): catálogo, inventário, pedido de compra — documentos de
 *   gestão/internos, com cabeçalho repetido por página e mais densidade de
 *   informação.
 */

const BRAND = '#1AA886'
const BRAND_DARK = '#0e7259'

function esc(text: string | number | null | undefined): string {
    return String(text ?? '')
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
}

/** `tenantSlug` chega cru ("auto-pecas-silva") — capitaliza para o cabeçalho. */
function prettifyStoreName(name: string): string {
    return name
        .split(/[-_]+/)
        .filter(Boolean)
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
        .join(' ')
}

function openPrintWindow(title: string, css: string, bodyHtml: string, winWidth: number, winHeight: number) {
    const win = window.open('', '_blank', `width=${winWidth},height=${winHeight}`)
    if (!win) return
    win.document.write(
        `<!doctype html><html><head><meta charset="utf-8"><title>${esc(title)}</title>` +
            `<style>${css}</style></head><body>${bodyHtml}</body></html>`,
    )
    win.document.close()
    win.focus()
    // Dá um tick para o layout assentar antes de abrir o diálogo de impressão.
    setTimeout(() => {
        win.print()
        win.close()
    }, 250)
}

// ── Cupons (80mm térmico) ───────────────────────────────────────────────────

const RECEIPT_CSS = `
    @page { size: 80mm auto; margin: 3mm 4mm; }
    * { margin: 0; padding: 0; box-sizing: border-box; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
    body {
        width: 72mm;
        font-family: 'Courier New', Courier, monospace;
        font-size: 12px;
        font-weight: 600;
        line-height: 1.45;
        color: #000;
        /* Impressoras térmicas de baixa resolução "perdem" traços finos —
           forçamos peso/contraste altos em todo o cupom em vez de depender
           do peso padrão do navegador. */
        -webkit-font-smoothing: antialiased;
    }
    .center { text-align: center; }
    .right { text-align: right; }
    .bold { font-weight: 800; }
    .store-name { font-size: 17px; font-weight: 800; letter-spacing: 0.5px; }
    .doc-title {
        margin: 5px 0;
        padding: 3px 0;
        border-top: 2px solid #000;
        border-bottom: 2px solid #000;
        font-size: 14px;
        font-weight: 800;
        letter-spacing: 1px;
    }
    .sm { font-size: 10.5px; font-weight: 600; color: #000; }
    .rule { border-top: 1px dashed #000; margin: 5px 0; }
    .rule-solid { border-top: 2px solid #000; margin: 5px 0; }
    .meta-row { display: flex; justify-content: space-between; gap: 6px; font-size: 11px; font-weight: 600; padding: 1px 0; }
    .meta-row .label { color: #000; }
    .meta-row .value { font-weight: 800; text-align: right; }
    .item { margin: 4px 0; }
    .item-desc { font-size: 12px; font-weight: 700; }
    .item-sku { display: block; font-size: 10px; font-weight: 600; color: #000; }
    .item-line {
        display: flex;
        align-items: baseline;
        gap: 4px;
        font-size: 11.5px;
        font-weight: 600;
        margin-top: 1px;
    }
    .item-qty { white-space: nowrap; color: #000; }
    .item-leader { flex: 1; border-bottom: 2px dotted #000; margin-bottom: 3px; }
    .item-value { white-space: nowrap; font-weight: 800; }
    .totals { margin-top: 4px; }
    .totals-row { display: flex; justify-content: space-between; font-size: 12px; font-weight: 700; padding: 1.5px 0; }
    .totals-row.muted { color: #000; }
    .totals-row.grand {
        font-size: 16px;
        font-weight: 800;
        border-top: 2px solid #000;
        margin-top: 3px;
        padding-top: 4px;
    }
    .payment {
        margin-top: 6px;
        padding: 4px 0;
        border-top: 2px dashed #000;
        border-bottom: 2px dashed #000;
        text-align: center;
        font-size: 11.5px;
        font-weight: 700;
    }
    .footer { margin-top: 10px; text-align: center; font-size: 10.5px; font-weight: 600; color: #000; }
    .footer .thanks { font-size: 13px; font-weight: 800; color: #000; margin-bottom: 3px; }
    .cut { margin-top: 10px; text-align: center; font-size: 10.5px; font-weight: 600; color: #000; white-space: nowrap; }
`

export interface ReceiptItem {
    descricao: string
    sku?: string
    quantidade: number
    unitCents: number
}

/** Linha de metadados exibida sob o cabeçalho (ex.: "Cliente", "Emitido em"). */
export interface ReceiptMetaLine {
    label: string
    value: string
}

/** Dados da empresa (Configurações → Dados da empresa) — todos opcionais,
 * campo ausente/vazio simplesmente não aparece no recibo. */
export interface BusinessInfo {
    cnpj?: string | null
    telefone?: string | null
    endereco?: string | null
    chavePix?: string | null
    informacoesAdicionais?: string | null
}

export interface ReceiptData {
    storeName: string
    title: string
    reference?: string
    /** Linhas de contexto (data, cliente, vendedor...) — mostradas em ordem. */
    meta?: ReceiptMetaLine[]
    /** Dados da empresa (CNPJ/telefone/endereço no cabeçalho, PIX/observações no rodapé). */
    businessInfo?: BusinessInfo
    items: ReceiptItem[]
    /** Quando informado junto de `totalCents`, mostra a quebra Subtotal/Desconto/Total. */
    subtotalCents?: number
    discountCents?: number
    totalCents: number
    paymentLabel?: string
    footerNote?: string
}

export interface InventoryReportRow {
    sku: string
    descricao: string
    categoria?: string
    quantidade: number
    custoMedioCents: number
    precoCustoCents?: number
    precoVendaCents?: number
}

export interface CatalogReportRow {
    sku: string
    descricao: string
    categoria: string
    precoVendaCents: number
}

export interface PurchaseOrderData {
    storeName: string
    numero: string
    status: string
    fornecedorNome: string
    fornecedorCnpj?: string
    prazoPagamentoDias: number
    items: { sku?: string; descricao: string; quantidade: number; unitCents: number }[]
    totalCents: number
}

export function useThermalPrint() {
    const { formatCentavos } = useFormat()

    function businessInfoHeaderLines(info?: BusinessInfo): string {
        if (!info) return ''
        const lines = [info.endereco, [info.telefone, info.cnpj ? `CNPJ ${info.cnpj}` : null].filter(Boolean).join(' · ')]
            .map((l) => l?.trim())
            .filter((l): l is string => !!l)
        return lines.map((l) => `<div class="center sm">${esc(l)}</div>`).join('')
    }

    function receiptHeader(storeName: string, title: string, reference?: string, businessInfo?: BusinessInfo): string {
        const now = new Date().toLocaleString('pt-BR', { dateStyle: 'short', timeStyle: 'short' })
        return (
            `<div class="center store-name">${esc(prettifyStoreName(storeName).toUpperCase())}</div>` +
            businessInfoHeaderLines(businessInfo) +
            `<div class="center doc-title">${esc(title)}</div>` +
            (reference ? `<div class="center sm">${esc(reference)}</div>` : '') +
            `<div class="center sm">Emitido em ${esc(now)}</div>`
        )
    }

    function metaBlock(meta?: ReceiptMetaLine[]): string {
        if (!meta?.length) return ''
        return (
            `<div class="rule"></div>` +
            meta
                .filter((m) => m.value)
                .map((m) => `<div class="meta-row"><span class="label">${esc(m.label)}</span><span class="value">${esc(m.value)}</span></div>`)
                .join('')
        )
    }

    function itemsBlock(items: ReceiptItem[]): string {
        return items
            .map(
                (i) =>
                    `<div class="item">` +
                    `<div class="item-desc">${esc(i.descricao)}${i.sku ? `<span class="item-sku">${esc(i.sku)}</span>` : ''}</div>` +
                    `<div class="item-line">` +
                    `<span class="item-qty">${esc(i.quantidade)} × ${formatCentavos(i.unitCents)}</span>` +
                    `<span class="item-leader"></span>` +
                    `<span class="item-value">${formatCentavos(i.quantidade * i.unitCents)}</span>` +
                    `</div></div>`,
            )
            .join('')
    }

    function totalsBlock(data: ReceiptData): string {
        const hasBreakdown = data.subtotalCents != null && (data.discountCents ?? 0) > 0
        const rows = hasBreakdown
            ? `<div class="totals-row muted"><span>Subtotal</span><span>${formatCentavos(data.subtotalCents)}</span></div>` +
              `<div class="totals-row muted"><span>Desconto</span><span>−${formatCentavos(data.discountCents)}</span></div>`
            : ''
        return (
            `<div class="totals">${rows}` +
            `<div class="totals-row grand"><span>TOTAL</span><span>${formatCentavos(data.totalCents)}</span></div>` +
            `</div>`
        )
    }

    /** Recibo de venda / cupom PDV / orçamento (documento não fiscal, 80mm). */
    function printReceipt(data: ReceiptData) {
        const itemCount = data.items.reduce((s, i) => s + i.quantidade, 0)
        const pix = data.businessInfo?.chavePix?.trim()
        const infoAdicional = data.businessInfo?.informacoesAdicionais?.trim()
        const html =
            receiptHeader(data.storeName, data.title, data.reference, data.businessInfo) +
            metaBlock(data.meta) +
            `<div class="rule-solid"></div>` +
            itemsBlock(data.items) +
            `<div class="rule"></div>` +
            totalsBlock(data) +
            `<div class="sm center" style="margin-top:3px">${itemCount} ${itemCount === 1 ? 'item' : 'itens'}</div>` +
            (data.paymentLabel ? `<div class="payment">${esc(data.paymentLabel)}</div>` : '') +
            (pix ? `<div class="payment">Chave PIX: ${esc(pix)}</div>` : '') +
            `<div class="footer">` +
            `<div class="thanks">Obrigado pela preferência!</div>` +
            `<div>${esc(data.footerNote ?? 'Documento sem valor fiscal.')}</div>` +
            (infoAdicional ? `<div>${esc(infoAdicional)}</div>` : '') +
            `</div>` +
            `<div class="cut">- - - - - - - corte aqui - - - - - - -</div>`
        openPrintWindow(data.title, RECEIPT_CSS, html, 420, 640)
    }

    // ── Relatórios de gestão (A4) ────────────────────────────────────────────

    const A4_CSS = `
        @page { size: A4; margin: 16mm 14mm; }
        * { margin: 0; padding: 0; box-sizing: border-box; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
        body {
            font-family: 'Segoe UI', Helvetica, Arial, sans-serif;
            font-size: 11px;
            color: #1a1a1a;
            line-height: 1.4;
        }
        .letterhead {
            display: flex;
            justify-content: space-between;
            align-items: flex-end;
            border-bottom: 2.5px solid ${BRAND};
            padding-bottom: 8px;
            margin-bottom: 2px;
        }
        .letterhead .store-name { font-size: 20px; font-weight: 700; color: ${BRAND_DARK}; letter-spacing: 0.3px; }
        .letterhead .doc-title { font-size: 13px; font-weight: 600; color: #333; text-transform: uppercase; letter-spacing: 0.5px; }
        .letterhead-right { text-align: right; font-size: 10px; color: #666; }
        .subtitle { font-size: 10.5px; color: #555; margin: 8px 0 10px; }
        table.report { width: 100%; border-collapse: collapse; }
        table.report thead th {
            background: ${BRAND};
            color: #fff;
            font-size: 10px;
            text-transform: uppercase;
            letter-spacing: 0.3px;
            text-align: left;
            padding: 6px 8px;
            font-weight: 600;
        }
        table.report thead th.num, table.report td.num { text-align: right; }
        table.report tbody td {
            padding: 5px 8px;
            border-bottom: 1px solid #ececec;
            font-size: 10.5px;
            vertical-align: top;
        }
        table.report tbody tr:nth-child(even) td { background: #f7f9f8; }
        table.report tbody tr.group-row td {
            background: #eafaf4;
            color: ${BRAND_DARK};
            font-weight: 700;
            font-size: 10px;
            text-transform: uppercase;
            letter-spacing: 0.4px;
            padding-top: 9px;
            border-bottom: 1.5px solid ${BRAND};
        }
        .cell-sub { display: block; font-size: 9px; color: #777; font-weight: 400; }
        table.report tfoot td {
            padding: 8px 8px 2px;
            font-size: 10.5px;
            border-top: 2px solid #1a1a1a;
            font-weight: 700;
        }
        .doc-footer {
            margin-top: 14px;
            padding-top: 8px;
            border-top: 1px solid #ddd;
            display: flex;
            justify-content: space-between;
            font-size: 9px;
            color: #888;
        }
        .info-panel {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
            gap: 4px 18px;
            background: #f7f9f8;
            border: 1px solid #e4e4e4;
            border-left: 3px solid ${BRAND};
            padding: 10px 14px;
            margin: 10px 0 14px;
            font-size: 10.5px;
        }
        .info-panel .field-label { color: #777; font-size: 9px; text-transform: uppercase; letter-spacing: 0.3px; }
        .info-panel .field-value { font-weight: 600; color: #1a1a1a; }
        .sign-row { display: flex; justify-content: space-between; margin-top: 34px; gap: 24px; }
        .sign-box { flex: 1; text-align: center; font-size: 9.5px; color: #666; }
        .sign-box .line { border-top: 1px solid #999; margin-bottom: 4px; padding-top: 22px; }
    `

    function a4Letterhead(storeName: string, docTitle: string, subtitle?: string): string {
        const now = new Date().toLocaleString('pt-BR', { dateStyle: 'short', timeStyle: 'short' })
        return (
            `<div class="letterhead">` +
            `<div><div class="store-name">${esc(prettifyStoreName(storeName))}</div><div class="doc-title">${esc(docTitle)}</div></div>` +
            `<div class="letterhead-right">Gerado em<br>${esc(now)}</div>` +
            `</div>` +
            (subtitle ? `<div class="subtitle">${esc(subtitle)}</div>` : '')
        )
    }

    function a4Footer(extra?: string): string {
        return (
            `<div class="doc-footer">` +
            `<span>${esc(extra ?? 'Documento interno — não é documento fiscal.')}</span>` +
            `<span>Finledger · Sistema de gestão para o varejo</span>` +
            `</div>`
        )
    }

    /** Inventário valorizado a preço de custo médio, agrupado por categoria. */
    function printInventoryReport(storeName: string, rows: InventoryReportRow[]) {
        const totalUnits = rows.reduce((s, r) => s + r.quantidade, 0)
        const totalCents = rows.reduce((s, r) => s + r.quantidade * r.custoMedioCents, 0)

        const porCategoria = new Map<string, InventoryReportRow[]>()
        for (const r of rows) {
            const cat = r.categoria?.trim() || 'Sem categoria'
            porCategoria.set(cat, [...(porCategoria.get(cat) ?? []), r])
        }
        const groupsHtml = rows.some((r) => r.categoria)
            ? [...porCategoria.entries()]
                  .sort(([a], [b]) => a.localeCompare(b))
                  .map(([cat, itens]) => groupRow(cat) + itens.map(inventoryRow).join(''))
                  .join('')
            : rows.map(inventoryRow).join('')

        function groupRow(label: string): string {
            return `<tr class="group-row"><td colspan="6">${esc(label)}</td></tr>`
        }
        function inventoryRow(r: InventoryReportRow): string {
            return (
                `<tr><td>${esc(r.descricao)}<span class="cell-sub">${esc(r.sku)}</span></td>` +
                `<td class="num">${esc(r.quantidade)}</td>` +
                `<td class="num">${formatCentavos(r.custoMedioCents)}</td>` +
                `<td class="num">${r.precoCustoCents != null ? formatCentavos(r.precoCustoCents) : '—'}</td>` +
                `<td class="num">${r.precoVendaCents != null ? formatCentavos(r.precoVendaCents) : '—'}</td>` +
                `<td class="num">${formatCentavos(r.quantidade * r.custoMedioCents)}</td></tr>`
            )
        }

        const html =
            a4Letterhead(storeName, 'Inventário a Preço de Custo', `${rows.length} produto(s) com saldo em estoque`) +
            `<table class="report">` +
            `<thead><tr><th>Produto</th><th class="num">Qtd.</th><th class="num">Custo médio</th>` +
            `<th class="num">Preço custo</th><th class="num">Preço venda</th><th class="num">Valor total</th></tr></thead>` +
            `<tbody>${groupsHtml}</tbody>` +
            `<tfoot><tr><td>${rows.length} produto(s) · ${totalUnits} unidade(s)</td><td colspan="4"></td>` +
            `<td class="num">${formatCentavos(totalCents)}</td></tr></tfoot>` +
            `</table>` +
            a4Footer()
        openPrintWindow('Inventário', A4_CSS, html, 900, 700)
    }

    /** Catálogo de produtos com preços de venda, agrupado por categoria. */
    function printCatalogReport(storeName: string, rows: CatalogReportRow[]) {
        const porCategoria = new Map<string, CatalogReportRow[]>()
        for (const r of rows) {
            const cat = r.categoria || 'Sem categoria'
            porCategoria.set(cat, [...(porCategoria.get(cat) ?? []), r])
        }
        const body = [...porCategoria.entries()]
            .sort(([a], [b]) => a.localeCompare(b))
            .map(
                ([cat, itens]) =>
                    `<tr class="group-row"><td colspan="2">${esc(cat)} <span style="opacity:.65;font-weight:400">(${itens.length})</span></td></tr>` +
                    itens
                        .sort((a, b) => a.descricao.localeCompare(b.descricao))
                        .map(
                            (r) =>
                                `<tr><td>${esc(r.descricao)}<span class="cell-sub">${esc(r.sku)}</span></td>` +
                                `<td class="num">${formatCentavos(r.precoVendaCents)}</td></tr>`,
                        )
                        .join(''),
            )
            .join('')
        const html =
            a4Letterhead(storeName, 'Catálogo de Produtos', `${rows.length} produto(s) ativo(s)`) +
            `<table class="report">` +
            `<thead><tr><th>Produto</th><th class="num">Preço de venda</th></tr></thead>` +
            `<tbody>${body}</tbody>` +
            `<tfoot><tr><td>${rows.length} produto(s)</td><td></td></tr></tfoot>` +
            `</table>` +
            a4Footer('Preços sujeitos a alteração sem aviso prévio.')
        openPrintWindow('Catálogo', A4_CSS, html, 900, 700)
    }

    /** Pedido de compra formal (A4) — documento interno para conferência/envio ao fornecedor. */
    function printPurchaseOrderReport(data: PurchaseOrderData) {
        const rows = data.items
            .map(
                (i) =>
                    `<tr><td>${esc(i.descricao)}${i.sku ? `<span class="cell-sub">${esc(i.sku)}</span>` : ''}</td>` +
                    `<td class="num">${esc(i.quantidade)}</td>` +
                    `<td class="num">${formatCentavos(i.unitCents)}</td>` +
                    `<td class="num">${formatCentavos(i.quantidade * i.unitCents)}</td></tr>`,
            )
            .join('')
        const html =
            a4Letterhead(data.storeName, 'Pedido de Compra', `Nº ${data.numero} · Status: ${data.status}`) +
            `<div class="info-panel">` +
            `<div><div class="field-label">Fornecedor</div><div class="field-value">${esc(data.fornecedorNome)}</div></div>` +
            (data.fornecedorCnpj
                ? `<div><div class="field-label">CNPJ</div><div class="field-value">${esc(data.fornecedorCnpj)}</div></div>`
                : '') +
            `<div><div class="field-label">Prazo de pagamento</div><div class="field-value">${esc(data.prazoPagamentoDias)} dias</div></div>` +
            `<div><div class="field-label">Itens</div><div class="field-value">${data.items.length}</div></div>` +
            `</div>` +
            `<table class="report">` +
            `<thead><tr><th>Produto</th><th class="num">Qtd.</th><th class="num">Custo unit.</th><th class="num">Subtotal</th></tr></thead>` +
            `<tbody>${rows}</tbody>` +
            `<tfoot><tr><td colspan="3">TOTAL DO PEDIDO</td><td class="num">${formatCentavos(data.totalCents)}</td></tr></tfoot>` +
            `</table>` +
            `<div class="sign-row">` +
            `<div class="sign-box"><div class="line">Comprador</div></div>` +
            `<div class="sign-box"><div class="line">Recebido por / Data</div></div>` +
            `</div>` +
            a4Footer()
        openPrintWindow('Pedido de Compra', A4_CSS, html, 900, 700)
    }

    return { printReceipt, printInventoryReport, printCatalogReport, printPurchaseOrderReport }
}
