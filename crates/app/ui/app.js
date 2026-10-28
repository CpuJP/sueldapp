// Sueldapp — interfaz. No calcula dinero: muestra los textos que devuelve el núcleo en Rust.
"use strict";

const invoke = window.__TAURI__?.core?.invoke;
const $ = (sel, raiz = document) => raiz.querySelector(sel);
const $$ = (sel, raiz = document) => [...raiz.querySelectorAll(sel)];

const MESES = ["Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio", "Julio", "Agosto",
  "Septiembre", "Octubre", "Noviembre", "Diciembre"];

const hoy = new Date();
const estado = {
  vista: "tablero",
  anio: hoy.getFullYear(),
  mes: hoy.getMonth() + 1,
  modoResumen: "mes",
  cat: null,
  config: null,
  filtro: { texto: "", anio: null, mes: null, estado_id: null, categoria_id: null, deuda_id: null, tarjeta_id: null, pago: null },
  ultimoInforme: null,
  /** Página actual de cada lista (cursor o inicio), por clave. */
  paginas: {},
  /** Desplegables abiertos (cuotas de deudas, pagos de tarjetas), para conservarlos al recargar. */
  abiertos: new Set(),
};

// ---------- Utilidades ----------

function esc(v) {
  return String(v ?? "").replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

async function api(cmd, args = {}) {
  if (!invoke) throw new Error("La interfaz debe abrirse desde la app Sueldapp.");
  try {
    return await invoke(cmd, args);
  } catch (e) {
    throw new Error(typeof e === "string" ? e : e?.message || String(e));
  }
}

function aviso(texto, { error = false, deshacer = null } = {}) {
  const caja = document.createElement("div");
  caja.className = "aviso" + (error ? " error" : "");
  caja.setAttribute("role", error ? "alert" : "status");
  const t = document.createElement("span");
  t.textContent = texto;
  caja.append(t);
  if (deshacer) {
    const b = document.createElement("button");
    b.textContent = "Deshacer";
    b.addEventListener("click", async () => {
      caja.remove();
      try {
        await deshacer();
        aviso("Cambio deshecho.");
        await recargar();
      } catch (e) {
        aviso(e.message, { error: true });
      }
    });
    caja.append(b);
  }
  $("#avisos").append(caja);
  setTimeout(() => caja.remove(), deshacer ? 9000 : 5000);
}

const ICONOS = {
  anterior: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m15 5-7 7 7 7"/></svg>',
  siguiente: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m9 5 7 7-7 7"/></svg>',
  mas: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>',
  lapiz: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16z"/></svg>',
  check: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 5 5 9-10"/></svg>',
  reloj: '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8"/><path d="M12 8v4l3 2"/></svg>',
  x: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg>',
  pausa: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6v12M15 6v12"/></svg>',
  buscar: '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="6"/><path d="m20 20-4.5-4.5"/></svg>',
};

function chipEstado(g) {
  const n = (g.estado || "Sin estado").toLowerCase();
  let clase = "pendiente", icono = ICONOS.reloj;
  if (g.pagado) { clase = "pagado"; icono = ICONOS.check; }
  else if (n === "no") { clase = "no"; icono = ICONOS.x; }
  else if (n === "aplazado") { icono = ICONOS.pausa; }
  return `<span class="estado ${clase}">${icono}${esc(g.estado || "Sin estado")}</span>`;
}

function cifra(m, clase = "") {
  if (!m) return "—";
  return `<span class="cifra ${m.negativo ? "negativo" : ""} ${clase}">${esc(m.texto)}</span>`;
}

function etiquetas(g) {
  const v = [];
  if (g.categoria) v.push(g.categoria);
  if (g.deuda) v.push("Deuda: " + g.deuda);
  if (g.tarjeta) v.push("Tarjeta: " + g.tarjeta);
  if (g.fuera_de_bloque) v.push("No cuenta en el total");
  if (g.desglose) v.push("Desglose " + g.desglose);
  return v.length ? `<div class="etiquetas">${v.map((x) => `<span class="chip">${esc(x)}</span>`).join("")}</div>` : "";
}

function opciones(lista, seleccionado, vacio) {
  const o = vacio !== undefined ? [`<option value="">${esc(vacio)}</option>`] : [];
  for (const x of lista) {
    o.push(`<option value="${esc(x.id)}" ${String(x.id) === String(seleccionado ?? "") ? "selected" : ""}>${esc(x.nombre)}</option>`);
  }
  return o.join("");
}

const numOpc = (v) => (v === "" || v == null ? null : Number(v));

function acciones(html) {
  $("#barra-acciones").innerHTML = html;
}

// ---------- Montos: miles con punto, decimales con coma (máximo 2) ----------

/** Da formato mientras se escribe: "1234567,891" → "1.234.567,89". Solo presentación. */
function formatearEntrada(texto) {
  const t = String(texto ?? "").replace(/[^\d,]/g, "");
  const i = t.indexOf(",");
  let entero = (i >= 0 ? t.slice(0, i) : t).replace(/^0+(?=\d)/, "");
  const decimales = i >= 0 ? t.slice(i + 1).replace(/,/g, "").slice(0, 2) : null;
  if (i >= 0 && entero === "") entero = "0";
  entero = entero.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return decimales === null ? entero : `${entero},${decimales}`;
}

function mascaraMonto(input) {
  input.value = formatearEntrada(input.value);
  input.addEventListener("input", (e) => {
    let texto = input.value;
    let cursor = input.selectionStart ?? texto.length;
    // El punto se pone solo; si la persona escribe "." (teclado numérico) quiere decir coma.
    if (e.data === "." && !texto.slice(0, cursor - 1).concat(texto.slice(cursor)).includes(",")) {
      texto = texto.slice(0, cursor - 1) + "," + texto.slice(cursor);
    }
    const significativos = texto.slice(0, cursor).replace(/[^\d,]/g, "").length;
    const nuevo = formatearEntrada(texto);
    input.value = nuevo;
    let pos = 0;
    for (let n = 0; pos < nuevo.length && n < significativos; pos++) if (/[\d,]/.test(nuevo[pos])) n++;
    input.setSelectionRange(pos, pos);
  });
}

// ---------- Diálogos ----------

/** Formulario modal. `campos`: [{nombre, etiqueta, tipo, valor, opciones, ancho, ayuda, requerido}]. */
function formulario({ titulo, campos, guardar, borrar, textoGuardar = "Guardar" }) {
  const d = $("#dialogo");
  const html = campos.map((c, i) => {
    const id = `campo-${i}`;
    const ayuda = c.ayuda ? `<p class="ayuda" id="${id}-ayuda">${esc(c.ayuda)}</p>` : "";
    const desc = c.ayuda ? `aria-describedby="${id}-ayuda"` : "";
    const req = c.requerido ? "required" : "";
    if (c.tipo === "check") {
      return `<div class="campo check ancho"><input type="checkbox" id="${id}" name="${c.nombre}" ${c.valor ? "checked" : ""} ${desc}><label for="${id}">${esc(c.etiqueta)}</label>${ayuda}</div>`;
    }
    let control;
    if (c.tipo === "select") control = `<select id="${id}" name="${c.nombre}" ${req} ${desc}>${c.opciones}</select>`;
    else if (c.tipo === "area") control = `<textarea id="${id}" name="${c.nombre}" ${desc}>${esc(c.valor)}</textarea>`;
    else control = `<input id="${id}" name="${c.nombre}" type="${c.tipo === "monto" ? "text" : c.tipo || "text"}" value="${esc(c.valor)}" ${req} ${desc} ${c.tipo === "monto" ? 'inputmode="decimal" autocomplete="off" data-monto' : ""} ${c.min != null ? `min="${c.min}" max="${c.max}"` : ""}>`;
    return `<div class="campo ${c.ancho ? "ancho" : ""}"><label for="${id}">${esc(c.etiqueta)}</label>${control}${ayuda}</div>`;
  }).join("");
  d.innerHTML = `<form method="dialog" novalidate>
      <h2 id="dialogo-titulo">${esc(titulo)}</h2>
      <p class="error" role="alert" hidden></p>
      <div class="campos">${html}</div>
      <div class="pie">
        ${borrar ? '<button type="button" class="peligro izq" data-accion="borrar">Borrar</button>' : ""}
        <button type="button" data-accion="cancelar">Cancelar</button>
        <button type="submit" class="primario">${esc(textoGuardar)}</button>
      </div></form>`;
  const form = $("form", d);
  $$("[data-monto]", form).forEach(mascaraMonto);
  const error = $(".error", form);
  form.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    const datos = {};
    for (const c of campos) {
      const el = form.elements[c.nombre];
      datos[c.nombre] = c.tipo === "check" ? el.checked : el.value;
    }
    try {
      await guardar(datos);
      d.close();
      await recargar();
    } catch (e) {
      error.textContent = e.message;
      error.hidden = false;
      error.focus?.();
    }
  });
  $('[data-accion="cancelar"]', d).addEventListener("click", () => d.close());
  if (borrar) $('[data-accion="borrar"]', d).addEventListener("click", async () => {
    d.close();
    await borrar();
  });
  d.showModal();
  $("input, select, textarea", d)?.focus();
}

/** Confirmación antes de borrar. Devuelve true si se confirma. */
function confirmar(titulo, texto, boton = "Borrar") {
  return new Promise((resolver) => {
    const d = $("#dialogo");
    d.innerHTML = `<form method="dialog"><h2 id="dialogo-titulo">${esc(titulo)}</h2>
      <p>${esc(texto)}</p>
      <div class="pie"><button value="no" type="submit">Cancelar</button>
      <button value="si" type="submit" class="primario">${esc(boton)}</button></div></form>`;
    d.addEventListener("close", () => resolver(d.returnValue === "si"), { once: true });
    d.returnValue = "";
    d.showModal();
    $('button[value="no"]', d).focus();
  });
}

async function borrarConDeshacer({ pregunta, detalle, borrar, restaurar, hecho }) {
  if (!(await confirmar(pregunta, detalle))) return;
  try {
    const copia = await borrar();
    await recargar();
    aviso(hecho, { deshacer: () => restaurar(copia) });
  } catch (e) {
    aviso(e.message, { error: true });
  }
}

// ---------- Formularios de registros ----------

async function editarGasto(g = null, base = {}) {
  const c = estado.cat;
  const anio = g?.anio ?? base.anio ?? estado.anio;
  const mes = g?.mes ?? base.mes ?? estado.mes;
  const pendiente = c.estados.find((e) => e.nombre === "Pendiente")?.id ?? c.estados[0]?.id;
  formulario({
    titulo: g ? "Editar gasto" : "Nuevo gasto",
    campos: [
      { nombre: "concepto", etiqueta: "Concepto", valor: g?.concepto ?? "", ancho: true, requerido: true },
      { nombre: "monto", etiqueta: "Monto (COP)", tipo: "monto", valor: g ? g.monto_texto.replace("$ ", "") : "", requerido: true, ayuda: "Ejemplo: 150.000 o 150000,50" },
      { nombre: "estado_id", etiqueta: "Estado", tipo: "select", opciones: opciones(c.estados, g?.estado_id ?? pendiente) },
      { nombre: "mes", etiqueta: "Mes", tipo: "select", opciones: opciones(MESES.map((n, i) => ({ id: i + 1, nombre: n })), mes) },
      { nombre: "anio", etiqueta: "Año", tipo: "number", valor: anio, min: 2000, max: 2100 },
      { nombre: "categoria_id", etiqueta: "Categoría", tipo: "select", opciones: opciones(c.categorias, g?.categoria_id, "Sin categoría") },
      { nombre: "deuda_id", etiqueta: "Cuota de la deuda", tipo: "select", opciones: opciones(c.deudas, g?.deuda_id ?? base.deuda_id, "Ninguna") },
      { nombre: "tarjeta_id", etiqueta: "Pago de la tarjeta", tipo: "select", opciones: opciones(c.tarjetas, g?.tarjeta_id ?? base.tarjeta_id, "Ninguna") },
      { nombre: "nota", etiqueta: "Nota", tipo: "area", valor: g?.nota ?? "", ancho: true },
      { nombre: "fuera_de_bloque", etiqueta: "No contar en los totales del mes", tipo: "check", valor: g?.fuera_de_bloque ?? false },
    ],
    guardar: (d) => api("guardar_gasto", {
      gasto: {
        id: g?.id ?? null, anio: Number(d.anio), mes: Number(d.mes), concepto: d.concepto, monto: d.monto,
        estado_id: Number(d.estado_id), categoria_id: numOpc(d.categoria_id), deuda_id: numOpc(d.deuda_id),
        tarjeta_id: numOpc(d.tarjeta_id), nota: d.nota, fuera_de_bloque: d.fuera_de_bloque,
      },
    }),
    borrar: g && (() => borrarConDeshacer({
      pregunta: "¿Borrar este gasto?", detalle: `${g.concepto} · ${g.monto_texto} · ${g.periodo}`,
      borrar: () => api("borrar_gasto", { id: g.id }),
      restaurar: (copia) => api("restaurar_gasto", { gasto: copia }),
      hecho: "Gasto borrado.",
    })),
  });
}

async function marcarPagado(g) {
  const pagado = estado.cat.estados.find((e) => e.cuenta_pagado);
  if (!pagado) return aviso("No hay ningún estado que cuente como pagado. Revísalo en Catálogos.", { error: true });
  const anterior = g.estado_id;
  const datos = (estado_id) => ({
    gasto: { id: g.id, anio: g.anio, mes: g.mes, concepto: g.concepto, monto: g.monto, estado_id,
      categoria_id: g.categoria_id, deuda_id: g.deuda_id, tarjeta_id: g.tarjeta_id, nota: g.nota, fuera_de_bloque: g.fuera_de_bloque },
  });
  try {
    await api("guardar_gasto", datos(pagado.id));
    await recargar();
    aviso(`"${g.concepto}" marcado como ${pagado.nombre}.`, { deshacer: () => api("guardar_gasto", datos(anterior)) });
  } catch (e) {
    aviso(e.message, { error: true });
  }
}

function editarIngreso(i = null) {
  const c = estado.cat;
  formulario({
    titulo: i ? "Editar ingreso" : "Nuevo ingreso",
    campos: [
      { nombre: "fuente_id", etiqueta: "Fuente", tipo: "select", opciones: opciones(c.fuentes, i?.fuente_id) },
      { nombre: "monto", etiqueta: "Monto (COP)", tipo: "monto", valor: i ? i.monto_texto.replace("$ ", "") : "", requerido: true },
      { nombre: "mes", etiqueta: "Mes", tipo: "select", opciones: opciones(MESES.map((n, k) => ({ id: k + 1, nombre: n })), i ? null : estado.mes) },
      { nombre: "anio", etiqueta: "Año", tipo: "number", valor: estado.anio, min: 2000, max: 2100 },
      { nombre: "nota", etiqueta: "Nota", tipo: "area", valor: i?.nota ?? "", ancho: true },
    ],
    guardar: (d) => api("guardar_ingreso", {
      ingreso: { id: i?.id ?? null, anio: Number(d.anio), mes: Number(d.mes), fuente_id: Number(d.fuente_id), monto: d.monto, nota: d.nota },
    }),
    borrar: i && (() => borrarConDeshacer({
      pregunta: "¿Borrar este ingreso?", detalle: `${i.fuente} · ${i.monto_texto}`,
      borrar: () => api("borrar_ingreso", { id: i.id }),
      restaurar: (copia) => api("restaurar_ingreso", { ingreso: copia }),
      hecho: "Ingreso borrado.",
    })),
  });
  if (i) $("#dialogo select[name=mes]").value = String(estado.mes);
}

function editarMes(m, nombre) {
  formulario({
    titulo: "Editar " + nombre,
    campos: [{ nombre: "cerrado", etiqueta: "Mes cerrado (ya no esperas más movimientos)", tipo: "check", valor: m.cerrado }],
    guardar: (d) => api("guardar_mes", { mes: { ...m, cerrado: d.cerrado } }),
    borrar: () => borrarConDeshacer({
      pregunta: `¿Borrar ${nombre}?`, detalle: "Se borran también todos sus gastos e ingresos.",
      borrar: () => api("borrar_mes", { id: m.id }),
      restaurar: (copia) => api("restaurar_mes", { mes: copia }),
      hecho: `${nombre} borrado.`,
    }),
  });
}

function editarDeuda(d = null) {
  formulario({
    titulo: d ? "Editar deuda" : "Nueva deuda",
    campos: [
      { nombre: "nombre", etiqueta: "Nombre", valor: d?.nombre ?? "", ancho: true, requerido: true },
      { nombre: "total", etiqueta: "Total de la deuda (COP)", tipo: "monto", valor: d?.total_texto?.replace("$ ", "") ?? "", ayuda: "Déjalo vacío si no lo sabes." },
      { nombre: "ajuste", etiqueta: "Pagado sin cuota (COP)", tipo: "monto", valor: d ? d.ajuste_texto.replace("$ ", "") : "0", ayuda: "Pagos que no están como gasto." },
      { nombre: "notas", etiqueta: "Notas", tipo: "area", valor: d?.notas ?? "", ancho: true },
      { nombre: "saldada", etiqueta: "Deuda saldada", tipo: "check", valor: d?.saldada ?? false },
    ],
    guardar: (x) => api("guardar_deuda", { deuda: { id: d?.id ?? null, nombre: x.nombre, total: x.total, ajuste: x.ajuste, saldada: x.saldada, notas: x.notas } }),
    borrar: d && (() => borrarConDeshacer({
      pregunta: "¿Borrar esta deuda?", detalle: `${d.nombre}. Sus cuotas se conservan como gastos sin deuda.`,
      borrar: () => api("borrar_deuda", { id: d.id }),
      restaurar: (copia) => api("restaurar_deuda", { borrado: copia }),
      hecho: "Deuda borrada.",
    })),
  });
}

function editarTarjeta(t = null) {
  formulario({
    titulo: t ? "Editar tarjeta" : "Nueva tarjeta",
    campos: [
      { nombre: "nombre", etiqueta: "Nombre", valor: t?.nombre ?? "", ancho: true, requerido: true },
      { nombre: "cupo", etiqueta: "Cupo (COP)", tipo: "monto", valor: t?.cupo_texto?.replace("$ ", "") ?? "" },
      { nombre: "saldo", etiqueta: "Saldo actual (COP)", tipo: "monto", valor: t?.saldo_texto?.replace("$ ", "") ?? "", ayuda: "Lo que debes hoy en la tarjeta." },
      { nombre: "dia_corte", etiqueta: "Día de corte", tipo: "number", valor: t?.dia_corte ?? "", min: 1, max: 31 },
      { nombre: "dia_pago", etiqueta: "Día de pago", tipo: "number", valor: t?.dia_pago ?? "", min: 1, max: 31 },
    ],
    guardar: (x) => api("guardar_tarjeta", { tarjeta: { id: t?.id ?? null, nombre: x.nombre, cupo: x.cupo, saldo: x.saldo, dia_corte: numOpc(x.dia_corte), dia_pago: numOpc(x.dia_pago) } }),
    borrar: t && (() => borrarConDeshacer({
      pregunta: "¿Borrar esta tarjeta?", detalle: `${t.nombre}. Sus pagos se conservan como gastos.`,
      borrar: () => api("borrar_tarjeta", { id: t.id }),
      restaurar: (copia) => api("restaurar_tarjeta", { borrado: copia }),
      hecho: "Tarjeta borrada.",
    })),
  });
}

// ---------- Vistas ----------

function cambiarMes(delta) {
  let m = estado.mes + delta, a = estado.anio;
  if (m < 1) { m = 12; a--; }
  if (m > 12) { m = 1; a++; }
  estado.mes = m; estado.anio = a;
  render();
}

function barraResumen() {
  const etiqueta = estado.modoResumen === "mes" ? `${MESES[estado.mes - 1]} ${estado.anio}` : String(estado.anio);
  const delta = estado.modoResumen === "mes" ? 1 : 12;
  acciones(`
    <div class="segmentado" role="group" aria-label="Ver por">
      <button type="button" data-modo="mes" aria-pressed="${estado.modoResumen === "mes"}">Mes</button>
      <button type="button" data-modo="anio" aria-pressed="${estado.modoResumen === "anio"}">Año</button>
    </div>
    <div class="selector-mes">
      <button type="button" class="icono" data-mover="-${delta}" aria-label="Periodo anterior (Alt + flecha izquierda)">${ICONOS.anterior}</button>
      <strong aria-live="polite">${esc(etiqueta)}</strong>
      <button type="button" class="icono" data-mover="${delta}" aria-label="Periodo siguiente (Alt + flecha derecha)">${ICONOS.siguiente}</button>
    </div>
    <button type="button" data-hoy>Hoy</button>`);
  $$("[data-modo]").forEach((b) => b.addEventListener("click", () => { estado.modoResumen = b.dataset.modo; render(); }));
  $$("[data-mover]").forEach((b) => b.addEventListener("click", () => cambiarMes(Number(b.dataset.mover))));
  $("[data-hoy]").addEventListener("click", () => { estado.anio = hoy.getFullYear(); estado.mes = hoy.getMonth() + 1; render(); });
}

function vistaVacia() {
  return `<section class="panel vacio">
    <h2>Aún no hay datos</h2>
    <p>Importa tu hoja <b>Ingreso-Egreso.ods</b> para traer tus meses, gastos, sueldos y deudas. El archivo original no se modifica.</p>
    <button type="button" class="primario" data-ir="datos">Importar mi hoja</button>
  </section>`;
}

async function vistaResumen(v) {
  barraResumen();
  if (estado.config?.vacia) {
    v.innerHTML = vistaVacia();
    return;
  }
  if (estado.modoResumen === "anio") return vistaAnio(v);
  const r = await api("resumen_mes", { anio: estado.anio, mes: estado.mes });
  const t = r.totales;
  if (!r.mes) {
    v.innerHTML = `<section class="panel vacio"><h2>${esc(r.nombre)} no tiene datos</h2>
      <p>Añade un ingreso o un gasto para empezar este mes.</p>
      <button type="button" class="primario" data-nuevo-ingreso>Añadir ingreso</button>
      <button type="button" data-nuevo-gasto>Añadir gasto</button></section>`;
    $("[data-nuevo-ingreso]", v).addEventListener("click", () => editarIngreso());
    $("[data-nuevo-gasto]", v).addEventListener("click", () => editarGasto());
    return;
  }
  const avance = Number(t.total.valor) > 0 ? Math.round((Number(t.pagado.valor) / Number(t.total.valor)) * 100) : 0;
  v.innerHTML = `
    <section class="panel heroe" aria-labelledby="h-restante">
      <div>
        <div class="etiqueta" id="h-restante">Te queda este mes</div>
        <div class="grande cifra ${t.restante.negativo ? "negativo" : ""}">${esc(t.restante.texto)}</div>
        <p class="ayuda">${t.restante.negativo ? "Los gastos del mes superan tus ingresos." : "Ingresos menos todos los gastos del mes."}
          De lo ya pagado te queda <b class="cifra ${t.restante_pagado.negativo ? "negativo" : ""}">${esc(t.restante_pagado.texto)}</b>.</p>
        ${r.mes.cerrado ? '<span class="insignia">Mes cerrado</span>' : ""}
        <button type="button" data-editar-mes>${ICONOS.lapiz}Editar mes</button>
      </div>
      <div>
        <dl class="cifras">
          <div><dt>Ingresos</dt><dd>${cifra(t.ingreso)}</dd></div>
          <div><dt>Gastos del mes</dt><dd>${cifra(t.total)}</dd></div>
          <div><dt>Ya pagado</dt><dd>${cifra(t.pagado)}</dd></div>
          <div><dt>Por pagar</dt><dd>${cifra(t.pendiente)}</dd></div>
        </dl>
        <label class="oculto" for="avance">Avance de pagos del mes</label>
        <progress id="avance" class="progreso" max="100" value="${avance}">${avance} %</progress>
        <p class="ayuda">Has pagado el ${avance} % de los gastos del mes.</p>
      </div>
    </section>
    <div class="columnas">
      <section class="panel" aria-labelledby="h-proximos">
        <div class="cabecera-panel"><h2 id="h-proximos">Próximos pagos</h2><span class="ayuda">${r.proximos.length} sin pagar</span></div>
        <div data-lista="proximos"></div>
      </section>
      <section class="panel" aria-labelledby="h-ingresos">
        <div class="cabecera-panel"><h2 id="h-ingresos">Ingresos</h2>
          <button type="button" data-nuevo-ingreso>${ICONOS.mas}Añadir</button></div>
        <div data-lista="ingresos"></div>
      </section>
    </div>
    <section class="panel" aria-labelledby="h-gastos">
      <div class="cabecera-panel"><h2 id="h-gastos">Gastos de ${esc(r.nombre)}</h2>
        <button type="button" class="primario" data-nuevo-gasto>${ICONOS.mas}Nuevo gasto</button></div>
      <div data-lista="gastos-mes"></div>
    </section>`;
  $("[data-editar-mes]", v).addEventListener("click", () => editarMes(r.mes, r.nombre));
  const periodo = `${estado.anio}-${estado.mes}`;
  paginarLocal($('[data-lista="proximos"]', v), `proximos-${periodo}`, r.proximos, (slice) => `<ul class="lista">${slice.map((g, i) => `
      <li><div><div class="concepto">${esc(g.concepto)}</div>${etiquetas(g)}</div>
      <span class="monto cifra">${esc(g.monto_texto)}</span>
      <button type="button" data-pagar="${i}">${ICONOS.check}Marcar pagado</button></li>`).join("")}</ul>`,
    (cont, slice) => $$("[data-pagar]", cont).forEach((b) => b.addEventListener("click", () => marcarPagado(slice[Number(b.dataset.pagar)]))),
    '<p class="ayuda">Todo pagado este mes.</p>');
  paginarLocal($('[data-lista="ingresos"]', v), `ingresos-${periodo}`, r.ingresos, (slice) => `<ul class="lista">${slice.map((i, k) => `
      <li><div><div class="concepto">${esc(i.fuente)}</div>${i.desglose ? `<div class="sub">Desglose ${esc(i.desglose)}</div>` : ""}</div>
      <span class="monto cifra">${esc(i.monto_texto)}</span>
      <button type="button" class="icono" data-ingreso="${k}" aria-label="Editar ingreso ${esc(i.fuente)}">${ICONOS.lapiz}</button></li>`).join("")}</ul>`,
    (cont, slice) => $$("[data-ingreso]", cont).forEach((b) => b.addEventListener("click", () => editarIngreso(slice[Number(b.dataset.ingreso)]))),
    '<p class="ayuda">Sin ingresos registrados.</p>');
  $$("[data-nuevo-ingreso]", v).forEach((b) => b.addEventListener("click", () => editarIngreso()));
  $$("[data-nuevo-gasto]", v).forEach((b) => b.addEventListener("click", () => editarGasto()));
  await listaGastos($('[data-lista="gastos-mes"]', v), `gastos-${periodo}`, { anio: estado.anio, mes: estado.mes }, { conMes: false, total: t.total });
}

// ---------- Paginación ----------

function tamPagina() {
  const n = Number(leerAjuste("tamPagina"));
  return [10, 25, 50].includes(n) ? n : 10;
}

/** Controles de página. `info`: {desde, hasta, cantidad, hayAnterior, haySiguiente}. */
function paginador(info) {
  if (!info.cantidad) return "";
  const tam = tamPagina();
  return `<nav class="paginador" aria-label="Páginas">
    <span class="pag-rango" aria-live="polite">${info.desde}–${info.hasta} de ${info.cantidad}</span>
    <label class="pag-tam">Por página
      <select data-pag-tam>${[10, 25, 50].map((n) => `<option ${n === tam ? "selected" : ""}>${n}</option>`).join("")}</select></label>
    <div class="pag-botones">
      <button type="button" data-pag="anterior" ${info.hayAnterior ? "" : "disabled"} aria-label="Página anterior">${ICONOS.anterior}Anterior</button>
      <button type="button" data-pag="siguiente" ${info.haySiguiente ? "" : "disabled"} aria-label="Página siguiente">Siguiente${ICONOS.siguiente}</button>
    </div></nav>`;
}

function activarPaginador(cont, ir) {
  $$("[data-pag]", cont).forEach((b) => b.addEventListener("click", async () => {
    await ir(b.dataset.pag);
    $(`[data-pag="${b.dataset.pag}"]:not([disabled])`, cont)?.focus();
  }));
  $("[data-pag-tam]", cont)?.addEventListener("change", (e) => {
    guardarAjuste("tamPagina", e.target.value);
    estado.paginas = {};
    render();
  });
}

/** Lista de gastos paginada por cursor desde el núcleo. `clave` guarda la página entre recargas. */
async function listaGastos(cont, clave, filtro, { conMes = true, total = null, cabecera = null } = {}) {
  const pag = (estado.paginas[clave] ??= { cursor: null, direccion: null });
  const r = await api("buscar_gastos", { filtro: { ...filtro, cursor: pag.cursor, direccion: pag.direccion, limite: tamPagina() } });
  if (!r.gastos.length && pag.cursor) {
    estado.paginas[clave] = { cursor: null, direccion: null };
    return listaGastos(cont, clave, filtro, { conMes, total, cabecera });
  }
  cont.innerHTML = (cabecera ? cabecera(r) : "") + tablaGastos(r.gastos, total, conMes) + paginador({
    desde: r.desde, hasta: r.hasta, cantidad: r.cantidad,
    hayAnterior: !!r.cursor_anterior, haySiguiente: !!r.cursor_siguiente,
  });
  activarFilas(cont, r.gastos);
  activarPaginador(cont, async (dir) => {
    estado.paginas[clave] = { cursor: dir === "siguiente" ? r.cursor_siguiente : r.cursor_anterior, direccion: dir };
    await listaGastos(cont, clave, filtro, { conMes, total, cabecera });
  });
  return r;
}

/** Paginación de una lista ya cargada (listas cortas: deudas, catálogos, historial…).
 *  `pintar(slice, inicio)` devuelve HTML; `activar(cont, slice, inicio)` engancha eventos. */
function paginarLocal(cont, clave, items, pintar, activar = () => {}, vacio = "") {
  const tam = tamPagina();
  const pag = (estado.paginas[clave] ??= { inicio: 0 });
  if (pag.inicio >= items.length) pag.inicio = Math.max(0, items.length - tam);
  pag.inicio -= pag.inicio % tam;
  const slice = items.slice(pag.inicio, pag.inicio + tam);
  cont.innerHTML = (items.length ? pintar(slice, pag.inicio) : vacio) + paginador({
    desde: items.length ? pag.inicio + 1 : 0, hasta: pag.inicio + slice.length, cantidad: items.length,
    hayAnterior: pag.inicio > 0, haySiguiente: pag.inicio + tam < items.length,
  });
  activar(cont, slice, pag.inicio);
  activarPaginador(cont, (dir) => {
    pag.inicio = Math.max(0, pag.inicio + (dir === "siguiente" ? tam : -tam));
    paginarLocal(cont, clave, items, pintar, activar, vacio);
  });
}

function tablaGastos(gastos, total, conMes) {
  if (!gastos.length) return '<p class="ayuda">No hay gastos.</p>';
  return `<div class="tabla-scroll"><table>
    <thead><tr>${conMes ? "<th>Mes</th>" : ""}<th>Concepto</th><th>Estado</th><th class="monto">Monto</th></tr></thead>
    <tbody>${gastos.map((g, i) => `
      <tr class="fila-accion" tabindex="0" data-fila="${i}" aria-label="Editar ${esc(g.concepto)}, ${esc(g.monto_texto)}, ${esc(g.estado)}">
        ${conMes ? `<td>${esc(g.periodo)}</td>` : ""}
        <td><div class="concepto">${esc(g.concepto)}</div>${etiquetas(g)}</td>
        <td>${chipEstado(g)}</td>
        <td class="monto">${esc(g.monto_texto)}</td></tr>`).join("")}</tbody>
    ${total ? `<tfoot><tr><td colspan="${conMes ? 3 : 2}">Total</td><td class="monto">${cifra(total)}</td></tr></tfoot>` : ""}
  </table></div>`;
}

function activarFilas(raiz, gastos) {
  $$("[data-fila]", raiz).forEach((tr) => {
    const abrir = () => editarGasto(gastos[Number(tr.dataset.fila)]);
    tr.addEventListener("click", abrir);
    tr.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); abrir(); } });
  });
}

async function vistaAnio(v) {
  const r = await api("resumen_anio", { anio: estado.anio });
  const t = r.totales;
  v.innerHTML = `
    <section class="panel heroe">
      <div><div class="etiqueta">Te queda en ${r.anio}</div>
        <div class="grande cifra ${t.restante.negativo ? "negativo" : ""}">${esc(t.restante.texto)}</div>
        <p class="ayuda">Suma de los restantes de cada mes.</p></div>
      <dl class="cifras">
        <div><dt>Ingresos del año</dt><dd>${cifra(t.ingreso)}</dd></div>
        <div><dt>Gastos del año</dt><dd>${cifra(t.total)}</dd></div>
        <div><dt>Ya pagado</dt><dd>${cifra(t.pagado)}</dd></div>
        <div><dt>Por pagar</dt><dd>${cifra(t.pendiente)}</dd></div>
      </dl>
    </section>
    <section class="panel"><h2>Mes a mes</h2>
      <div class="tabla-scroll"><table>
        <thead><tr><th>Mes</th><th class="monto">Ingresos</th><th class="monto">Gastos</th><th class="monto">Pagado</th><th class="monto">Restante</th></tr></thead>
        <tbody>${r.meses.map((m) => `
          <tr class="fila-accion" tabindex="0" data-mes="${m.mes}" aria-label="Abrir ${esc(m.nombre)}">
            <td>${esc(m.nombre)}${m.existe ? "" : ' <span class="ayuda">(sin datos)</span>'}</td>
            <td class="monto">${cifra(m.totales.ingreso)}</td><td class="monto">${cifra(m.totales.total)}</td>
            <td class="monto">${cifra(m.totales.pagado)}</td><td class="monto">${cifra(m.totales.restante)}</td></tr>`).join("")}</tbody>
        <tfoot><tr><td>Total</td><td class="monto">${cifra(t.ingreso)}</td><td class="monto">${cifra(t.total)}</td><td class="monto">${cifra(t.pagado)}</td><td class="monto">${cifra(t.restante)}</td></tr></tfoot>
      </table></div></section>`;
  $$("[data-mes]", v).forEach((tr) => {
    const abrir = () => { estado.mes = Number(tr.dataset.mes); estado.modoResumen = "mes"; render(); };
    tr.addEventListener("click", abrir);
    tr.addEventListener("keydown", (e) => { if (e.key === "Enter") abrir(); });
  });
}

async function vistaMovimientos(v) {
  acciones(`<button type="button" class="primario" data-nuevo-gasto>${ICONOS.mas}Nuevo gasto</button>`);
  $("[data-nuevo-gasto]").addEventListener("click", () => editarGasto());
  const c = estado.cat, f = estado.filtro;
  const anios = [...new Set((await api("listar_meses")).map((m) => m.anio))].map((a) => ({ id: a, nombre: String(a) }));
  v.innerHTML = `
    <section class="panel" aria-label="Filtros">
      <form class="filtros" role="search" id="filtros">
        <div class="campo"><label for="f-texto">Buscar</label>
          <div class="buscador">${ICONOS.buscar}<input id="f-texto" name="texto" type="search" value="${esc(f.texto)}" placeholder="Concepto o nota"></div></div>
        <div class="campo"><label for="f-anio">Año</label><select id="f-anio" name="anio">${opciones(anios, f.anio, "Todos")}</select></div>
        <div class="campo"><label for="f-mes">Mes</label><select id="f-mes" name="mes">${opciones(MESES.map((n, i) => ({ id: i + 1, nombre: n })), f.mes, "Todos")}</select></div>
        <div class="campo"><label for="f-estado">Estado</label><select id="f-estado" name="estado_id">${opciones(c.estados, f.estado_id, "Todos")}</select></div>
        <div class="campo"><label for="f-pago">Pago</label><select id="f-pago" name="pago">${opciones([{ id: "pagados", nombre: "Solo pagados" }, { id: "pendientes", nombre: "Solo sin pagar" }], f.pago, "Todos")}</select></div>
        <div class="campo"><label for="f-categoria">Categoría</label><select id="f-categoria" name="categoria_id">${opciones(c.categorias, f.categoria_id, "Todas")}</select></div>
        <div class="campo"><label for="f-deuda">Deuda</label><select id="f-deuda" name="deuda_id">${opciones(c.deudas, f.deuda_id, "Todas")}</select></div>
        <div class="campo"><label for="f-tarjeta">Tarjeta</label><select id="f-tarjeta" name="tarjeta_id">${opciones(c.tarjetas, f.tarjeta_id, "Todas")}</select></div>
        <div class="campo"><label class="oculto" for="f-limpiar">Limpiar filtros</label><button type="button" id="f-limpiar">Limpiar filtros</button></div>
      </form>
    </section>
    <section class="panel" aria-live="polite" id="resultados"></section>`;
  const form = $("#filtros", v);
  const buscar = () => listaGastos($("#resultados", v), "movimientos", {
    texto: f.texto || null, anio: numOpc(f.anio), mes: numOpc(f.mes), estado_id: numOpc(f.estado_id),
    categoria_id: numOpc(f.categoria_id), deuda_id: numOpc(f.deuda_id), tarjeta_id: numOpc(f.tarjeta_id), pago: f.pago || null,
  }, { cabecera: (r) => `<div class="cabecera-panel"><h2>${r.cantidad} movimientos</h2><span class="ayuda">Suma: <b class="cifra">${esc(r.total.texto)}</b></span></div>` });
  let espera;
  form.addEventListener("input", (e) => {
    f[e.target.name] = e.target.value || null;
    if (e.target.name === "texto") f.texto = e.target.value;
    clearTimeout(espera);
    delete estado.paginas.movimientos; // un filtro nuevo vuelve a la primera página
    espera = setTimeout(buscar, e.target.name === "texto" ? 180 : 0);
  });
  $("#f-limpiar", v).addEventListener("click", () => {
    Object.keys(f).forEach((k) => (f[k] = k === "texto" ? "" : null));
    delete estado.paginas.movimientos;
    render();
  });
  await buscar();
}

async function vistaDeudas(v) {
  acciones(`<button type="button" class="primario" data-nueva>${ICONOS.mas}Nueva deuda</button>`);
  $("[data-nueva]").addEventListener("click", () => editarDeuda());
  const deudas = await api("listar_deudas");
  const activas = deudas.filter((d) => !d.saldada), saldadas = deudas.filter((d) => d.saldada);
  const fila = (d, i) => {
    const p = d.progreso != null ? Number(d.progreso) : null;
    const pend = d.pendiente
      ? (d.pendiente.negativo ? `Pagado de más: <b class="cifra">${esc(d.pendiente.texto.replace("-", ""))}</b>` : `Pendiente: <b class="cifra">${esc(d.pendiente.texto)}</b>`)
      : "Sin total definido";
    return `<article class="fila-detalle" aria-labelledby="deuda-${i}">
      <div class="linea">
        <div><h3 id="deuda-${i}">${esc(d.nombre)}</h3>${d.saldada ? '<span class="insignia">Saldada</span>' : ""}</div>
        <div>${p != null ? `<label class="oculto" for="prog-${i}">Progreso de ${esc(d.nombre)}</label>
          <progress id="prog-${i}" class="progreso" max="100" value="${p}">${p} %</progress>
          <div class="ayuda">${String(p).replace(".", ",")} % pagado</div>` : '<p class="ayuda">Añade el total para ver el progreso.</p>'}</div>
        <button type="button" class="icono" data-editar="${i}" aria-label="Editar deuda ${esc(d.nombre)}">${ICONOS.lapiz}</button>
      </div>
      <div class="datos">
        <span>Pagado: <b class="cifra">${esc(d.pagado.texto)}</b>${d.total_texto ? ` de <b class="cifra">${esc(d.total_texto)}</b>` : ""}</span>
        <span>${pend}</span>
        <span>Cuotas pagadas: <b>${d.cuotas_pagadas} de ${d.cuotas_registradas}</b></span>
        <span>Fin estimado: <b>${esc(d.fin_estimado ?? "—")}</b></span>
      </div>
      <details data-detalle-deuda="${i}" ${estado.abiertos.has(`deuda-${d.id}`) ? "open" : ""}><summary>Ver cuotas (${d.cuotas.length})</summary>
        <div data-cuotas></div>
        <button type="button" data-cuota="${i}">${ICONOS.mas}Añadir cuota</button></details>
    </article>`;
  };
  const todas = [...activas, ...saldadas];
  v.innerHTML = !deudas.length ? '<section class="panel vacio"><h2>No hay deudas</h2><p>Crea una deuda para seguir sus cuotas.</p></section>' : `
    <section class="panel" aria-labelledby="h-activas"><h2 id="h-activas">En curso (${activas.length})</h2>
      <div class="filas-detalle" data-lista="activas"></div></section>
    <section class="panel" aria-labelledby="h-saldadas"><h2 id="h-saldadas">Saldadas (${saldadas.length})</h2>
      <div class="filas-detalle" data-lista="saldadas"></div></section>`;
  if (!deudas.length) return;
  const activar = (cont) => {
    $$("[data-editar]", cont).forEach((b) => b.addEventListener("click", () => editarDeuda(todas[Number(b.dataset.editar)])));
    $$("[data-cuota]", cont).forEach((b) => b.addEventListener("click", () => editarGasto(null, { deuda_id: todas[Number(b.dataset.cuota)].id })));
    $$("[data-detalle-deuda]", cont).forEach((det) => {
      const d = todas[Number(det.dataset.detalleDeuda)];
      const cargar = () => listaGastos($("[data-cuotas]", det), `cuotas-${d.id}`, { deuda_id: d.id });
      det.addEventListener("toggle", () => {
        estado.abiertos[det.open ? "add" : "delete"](`deuda-${d.id}`);
        if (det.open) cargar();
      });
      if (det.open) cargar();
    });
  };
  const pintar = (slice) => slice.map((d) => fila(d, todas.indexOf(d))).join("");
  paginarLocal($('[data-lista="activas"]', v), "deudas-activas", activas, pintar, activar, '<p class="ayuda">Ninguna.</p>');
  paginarLocal($('[data-lista="saldadas"]', v), "deudas-saldadas", saldadas, pintar, activar, '<p class="ayuda">Ninguna.</p>');
}

async function vistaTarjetas(v) {
  acciones(`<button type="button" class="primario" data-nueva>${ICONOS.mas}Nueva tarjeta</button>`);
  $("[data-nueva]").addEventListener("click", () => editarTarjeta());
  const tarjetas = await api("listar_tarjetas");
  v.innerHTML = !tarjetas.length ? '<section class="panel vacio"><h2>No hay tarjetas</h2><p>Crea una tarjeta de crédito para seguir sus pagos.</p></section>' : `
    <section class="panel"><div class="filas-detalle" data-lista="tarjetas"></div></section>`;
  if (!tarjetas.length) return;
  const pintar = (slice, inicio) => slice.map((t, k) => {
      const i = inicio + k;
      const uso = t.cupo && t.saldo ? Math.min(100, Math.round((Number(t.saldo) / Number(t.cupo)) * 100)) : null;
      return `<article class="fila-detalle" aria-labelledby="tarjeta-${i}">
        <div class="linea">
          <div><h3 id="tarjeta-${i}">${esc(t.nombre)}</h3><div class="ayuda">${t.pagos_registrados} pagos registrados</div></div>
          <div>${uso != null ? `<label class="oculto" for="uso-${i}">Cupo usado de ${esc(t.nombre)}</label>
            <progress id="uso-${i}" class="progreso uso" max="100" value="${uso}">${uso} %</progress>
            <div class="ayuda">${uso} % del cupo usado</div>` : '<p class="ayuda">Añade cupo y saldo para ver el uso.</p>'}</div>
          <button type="button" class="icono" data-editar="${i}" aria-label="Editar tarjeta ${esc(t.nombre)}">${ICONOS.lapiz}</button>
        </div>
        <div class="datos">
          <span>Cupo: <b class="cifra">${esc(t.cupo_texto ?? "—")}</b></span>
          <span>Saldo: <b class="cifra">${esc(t.saldo_texto ?? "—")}</b></span>
          <span>Disponible: ${t.disponible ? `<b>${cifra(t.disponible)}</b>` : "<b>—</b>"}</span>
          <span>Corte: <b>${t.dia_corte ? "día " + t.dia_corte : "—"}</b></span>
          <span>Pago: <b>${t.dia_pago ? "día " + t.dia_pago : "—"}</b></span>
          <span>Total pagado: <b class="cifra">${esc(t.total_pagado.texto)}</b></span>
        </div>
        <details data-detalle-tarjeta="${i}" ${estado.abiertos.has(`tarjeta-${t.id}`) ? "open" : ""}><summary>Ver pagos (${t.pagos.length})</summary><div data-pagos></div>
          <button type="button" data-pago="${i}">${ICONOS.mas}Añadir pago</button></details>
      </article>`;
    }).join("");
  paginarLocal($('[data-lista="tarjetas"]', v), "tarjetas", tarjetas, pintar, (cont) => {
    $$("[data-editar]", cont).forEach((b) => b.addEventListener("click", () => editarTarjeta(tarjetas[Number(b.dataset.editar)])));
    $$("[data-pago]", cont).forEach((b) => b.addEventListener("click", () => editarGasto(null, { tarjeta_id: tarjetas[Number(b.dataset.pago)].id })));
    $$("[data-detalle-tarjeta]", cont).forEach((det) => {
      const t = tarjetas[Number(det.dataset.detalleTarjeta)];
      const cargar = () => listaGastos($("[data-pagos]", det), `pagos-${t.id}`, { tarjeta_id: t.id });
      det.addEventListener("toggle", () => {
        estado.abiertos[det.open ? "add" : "delete"](`tarjeta-${t.id}`);
        if (det.open) cargar();
      });
      if (det.open) cargar();
    });
  });
}

async function vistaCatalogos(v) {
  acciones("");
  const c = estado.cat;
  const seccion = (tipo, titulo, lista, ayuda) => `
    <section class="panel" aria-labelledby="h-${tipo}"><h2 id="h-${tipo}">${titulo}</h2><p class="ayuda">${ayuda}</p>
      <div data-lista="cat-${tipo}"></div>
      <form class="form-linea" data-alta="${tipo}"><label class="oculto" for="alta-${tipo}">Nuevo elemento en ${titulo}</label>
        <input id="alta-${tipo}" name="nombre" placeholder="Nuevo…" required><button type="submit">${ICONOS.mas}Añadir</button></form>
    </section>`;
  v.innerHTML = `<div class="catalogos">
    ${seccion("estados", "Estados", c.estados, "Los estados que cuentan como pagado suman en «Ya pagado».")}
    ${seccion("categorias", "Categorías", c.categorias, "Para agrupar y filtrar gastos.")}
    ${seccion("fuentes", "Fuentes de ingreso", c.fuentes, "Sueldo, prima y otros ingresos.")}</div>`;
  $$("[data-alta]", v).forEach((form) => form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const tipo = form.dataset.alta, nombre = form.elements.nombre.value;
    try {
      if (tipo === "estados") await api("guardar_estado", { estado: { id: null, nombre, cuenta_pagado: false, orden: 50 } });
      else await api("guardar_item", { tipo, item: { id: null, nombre } });
      await recargar();
      aviso(`"${nombre}" añadido.`);
      $(`#alta-${tipo}`)?.focus();
    } catch (err) { aviso(err.message, { error: true }); }
  }));
  for (const [tipo, lista] of [["estados", c.estados], ["categorias", c.categorias], ["fuentes", c.fuentes]]) {
    paginarLocal($(`[data-lista="cat-${tipo}"]`, v), `cat-${tipo}`, lista, (slice) => `<ul class="lista">${slice.map((x) => `<li><div><div class="concepto">${esc(x.nombre)}</div>
        ${tipo === "estados" ? `<div class="sub">${x.cuenta_pagado ? "Cuenta como pagado" : "No cuenta como pagado"}</div>` : ""}</div>
        <span></span>
        <button type="button" class="icono" data-tipo="${tipo}" data-id="${x.id}" aria-label="Editar ${esc(x.nombre)}">${ICONOS.lapiz}</button></li>`).join("")}</ul>`,
      (cont) => $$("[data-tipo]", cont).forEach((b) => b.addEventListener("click", () => editarItem(b.dataset.tipo, Number(b.dataset.id)))));
  }
}

function editarItem(tipo, id) {
  const lista = tipo === "estados" ? estado.cat.estados : estado.cat[tipo];
  const x = lista.find((i) => i.id === id);
  const otros = lista.filter((i) => i.id !== id);
  const campos = [{ nombre: "nombre", etiqueta: "Nombre", valor: x.nombre, ancho: true, requerido: true }];
  if (tipo === "estados") {
    campos.push({ nombre: "cuenta_pagado", etiqueta: "Cuenta como pagado", tipo: "check", valor: x.cuenta_pagado });
  }
  campos.push({ nombre: "reasignar", etiqueta: "Si lo borras, sus movimientos pasan a", tipo: "select", ancho: true,
    opciones: opciones(otros, null, tipo === "categorias" ? "Sin categoría" : "Elige…"), ayuda: "Solo se usa al borrar." });
  formulario({
    titulo: "Editar " + x.nombre,
    campos,
    guardar: (d) => tipo === "estados"
      ? api("guardar_estado", { estado: { ...x, nombre: d.nombre, cuenta_pagado: d.cuenta_pagado } })
      : api("guardar_item", { tipo, item: { id, nombre: d.nombre } }),
    borrar: async () => {
      const destino = numOpc($("#dialogo select[name=reasignar]")?.value);
      if (!(await confirmar(`¿Borrar «${x.nombre}»?`, "Esta acción no se puede deshacer."))) return;
      try {
        if (tipo === "estados") await api("borrar_estado", { id, reasignarA: destino });
        else await api("borrar_item", { tipo, id, reasignarA: destino });
        await recargar();
        aviso(`«${x.nombre}» borrado.`);
      } catch (e) { aviso(e.message, { error: true }); }
    },
  });
}

function informeHtml(inf) {
  if (!inf) return "";
  const cc = inf.conciliacion;
  const diff = (lista) => `<table><thead><tr><th>Valor</th><th class="monto">Hoja</th><th class="monto">App</th></tr></thead>
    <tbody>${lista.map((d) => `<tr><td>${esc(d.concepto)}</td><td class="monto">${esc(d.hoja ?? "—")}</td><td class="monto">${esc(d.app ?? "—")}</td></tr>`).join("")}</tbody></table>`;
  const n = (acc) => inf.log.filter((l) => l.accion === acc).length;
  return `<section class="panel" aria-labelledby="h-informe">
    <h2 id="h-informe">Informe de importación</h2>
    <p class="${inf.ok ? "resultado-ok" : "resultado-mal"}">${inf.ya_importado ? "Este archivo ya estaba importado: no se cambió nada." : inf.ok ? "Importación correcta: los totales coinciden con la hoja." : "Importación fallida: hay diferencias y no se guardó nada."}</p>
    <div class="datos fila-detalle"><div class="datos">
      <span>Archivo: <b>${esc(inf.archivo)}</b> (${esc(inf.formato)})</span>
      <span>Meses: <b>${inf.conteos.meses}</b></span><span>Gastos: <b>${inf.conteos.gastos}</b></span>
      <span>Ingresos: <b>${inf.conteos.ingresos}</b></span><span>Deudas: <b>${inf.conteos.deudas}</b></span>
      <span>Valores comparados: <b>${cc.comparados}</b></span><span>Diferencias: <b>${cc.diferencias.length}</b></span></div></div>
    ${cc.diferencias.length ? `<h3>Diferencias</h3>${diff(cc.diferencias)}` : ""}
    ${cc.aceptadas.length ? `<h3>Diferencias aceptadas por ti</h3><p class="ayuda">Deudas sin fórmula de pago en la hoja: su total se calcula con sus cuotas (decisión tuya).</p>${diff(cc.aceptadas)}` : ""}
    <details><summary>Registro detallado: ${n("importada")} importadas, ${n("transformada")} transformadas, ${n("rechazada")} rechazadas, ${n("omitida")} omitidas</summary>
      <div class="log"><table><thead><tr><th>Celda</th><th>Acción</th><th>Detalle</th></tr></thead><tbody>
      ${inf.log.map((l) => `<tr><td>${esc(l.origen)}</td><td>${esc(l.accion)}</td><td>${esc(l.detalle)}</td></tr>`).join("")}</tbody></table></div></details>
  </section>`;
}

async function vistaDatos(v) {
  acciones("");
  const historial = await api("historial_importaciones");
  v.innerHTML = `
    <section class="panel" aria-labelledby="h-importar">
      <h2 id="h-importar">Importar</h2>
      <p class="ayuda">Elige tu hoja original, una plantilla llena o una exportación de Sueldapp. El archivo solo se lee.
        Si algo no cuadra, no se guarda nada. Importar una <b>exportación</b> de Sueldapp reemplaza todos los datos;
        una <b>plantilla</b> los suma.</p>
      <div class="ajuste-fila"><label for="forzar">Volver a importar aunque el archivo ya se haya importado</label>
        <input type="checkbox" id="forzar"></div>
      <button type="button" class="primario" id="importar">Elegir archivo e importar</button>
    </section>
    <div id="informe">${informeHtml(estado.ultimoInforme)}</div>
    <section class="panel" aria-labelledby="h-plantilla">
      <h2 id="h-plantilla">Plantilla para llenar a mano</h2>
      <p class="ayuda">Descarga una hoja de cálculo con las columnas listas (gastos, ingresos, deudas y tarjetas) y las
        instrucciones. Llénala en LibreOffice, Excel o Google Sheets, guárdala como <b>.ods</b> y cárgala arriba con
        <b>Elegir archivo e importar</b>. Suma datos a los que ya tienes; no borra nada.</p>
      <button type="button" id="plantilla">Descargar plantilla</button>
    </section>
    <section class="panel" aria-labelledby="h-exportar">
      <h2 id="h-exportar">Exportar</h2>
      <p class="ayuda">Guarda una copia de todos tus datos para abrirla en cualquier hoja de cálculo o para volver a importarla.</p>
      <div class="barra-acciones"><button type="button" id="exp-ods">Exportar a .ods</button>
        <button type="button" id="exp-csv">Exportar a .csv (una carpeta)</button></div>
    </section>
    <section class="panel" aria-labelledby="h-historial"><h2 id="h-historial">Historial de importaciones</h2>
      <div data-lista="historial"></div>
    </section>`;
  paginarLocal($('[data-lista="historial"]', v), "historial", historial, (slice) => `<ul class="lista">${slice.map((h) => `<li><div><div class="concepto">${esc(h.archivo)}</div><div class="sub">${esc(h.fecha)} · ${esc(h.resumen)}</div></div>
        <span class="estado ${h.ok ? "pagado" : "no"}">${h.ok ? ICONOS.check + "Correcta" : ICONOS.x + "Fallida"}</span><span></span></li>`).join("")}</ul>`,
    () => {}, '<p class="ayuda">Aún no hay importaciones.</p>');
  $("#importar", v).addEventListener("click", async () => {
    const forzar = $("#forzar", v).checked;
    try {
      const inf = await api("importar_ods", { forzar });
      if (!inf) return;
      if (inf.formato !== "hoja original" && !inf.ya_importado && !inf.ok) aviso("La exportación no cuadra; no se cambió nada.", { error: true });
      estado.ultimoInforme = inf;
      await recargar();
      aviso(inf.ok ? "Importación terminada." : "La importación falló; revisa el informe.", { error: !inf.ok });
      $("#h-informe")?.scrollIntoView({ block: "start" });
    } catch (e) { aviso(e.message, { error: true }); }
  });
  $("#plantilla", v).addEventListener("click", async () => {
    try { const r = await api("descargar_plantilla"); if (r) aviso("Plantilla guardada en " + r); } catch (e) { aviso(e.message, { error: true }); }
  });
  $("#exp-ods", v).addEventListener("click", async () => {
    try { const r = await api("exportar_ods"); if (r) aviso("Exportado en " + r); } catch (e) { aviso(e.message, { error: true }); }
  });
  $("#exp-csv", v).addEventListener("click", async () => {
    try { const r = await api("exportar_csv"); if (r) aviso(`${r.length} archivos CSV guardados.`); } catch (e) { aviso(e.message, { error: true }); }
  });
}

function leerAjuste(clave) {
  try { return localStorage.getItem(clave); } catch { return null; }
}
function guardarAjuste(clave, valor) {
  try { localStorage.setItem(clave, valor); } catch { /* sin almacenamiento: solo esta sesión */ }
}

function temaElegido() {
  const t = leerAjuste("tema");
  return t === "claro" || t === "oscuro" ? t : "sistema";
}

function aplicarAjustes() {
  const raiz = document.documentElement;
  if (leerAjuste("transparencia") === "reducida") raiz.dataset.transparencia = "reducida";
  else delete raiz.dataset.transparencia;
  if (leerAjuste("movimiento") === "reducido") raiz.dataset.movimiento = "reducido";
  else delete raiz.dataset.movimiento;
  // SUELDAPP_TEMA (capturas) manda; si no, lo que eligió la persona; "sistema" sigue al sistema.
  const tema = estado.config?.tema || temaElegido();
  if (tema === "claro" || tema === "oscuro") raiz.dataset.tema = tema;
  else delete raiz.dataset.tema;
}

async function vistaAjustes(v) {
  acciones("");
  const transp = leerAjuste("transparencia") === "reducida";
  const mov = leerAjuste("movimiento") === "reducido";
  v.innerHTML = `<section class="panel" aria-labelledby="h-apariencia"><h2 id="h-apariencia">Apariencia</h2>
      <div class="ajuste-fila"><div><label for="aj-transp">Reducir transparencia</label>
        <p class="ayuda">Cambia el vidrio por superficies sólidas. Mejora la lectura.</p></div>
        <input type="checkbox" role="switch" class="interruptor" id="aj-transp" ${transp ? "checked" : ""}></div>
      <div class="ajuste-fila"><div><label for="aj-mov">Reducir movimiento</label>
        <p class="ayuda">Quita animaciones. Si tu sistema ya lo pide, la app lo respeta siempre.</p></div>
        <input type="checkbox" role="switch" class="interruptor" id="aj-mov" ${mov ? "checked" : ""}></div>
      <div class="ajuste-fila"><div><span class="etiqueta-ajuste" id="aj-tema">Tema</span>
        <p class="ayuda">Claro, oscuro o el mismo de tu sistema.</p></div>
        <div class="selector-tema" role="radiogroup" aria-labelledby="aj-tema">
          ${[["sistema", "Sistema", '<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/></svg>'],
             ["claro", "Claro", '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>'],
             ["oscuro", "Oscuro", '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/></svg>']]
            .map(([v, n, icono]) => `<label class="opcion-tema"><input type="radio" name="tema" value="${v}" ${temaElegido() === v ? "checked" : ""}>${icono}<span>${n}</span></label>`).join("")}
        </div></div>
    </section>
    <section class="panel" aria-labelledby="h-datos-locales"><h2 id="h-datos-locales">Tus datos</h2>
      <p class="ayuda">Todo se guarda solo en este equipo. La app no se conecta a internet.</p>
      <p>Base de datos: <b>${esc(estado.config?.base)}</b></p></section>`;
  $("#aj-transp", v).addEventListener("change", (e) => { guardarAjuste("transparencia", e.target.checked ? "reducida" : ""); aplicarAjustes(); });
  $$('input[name="tema"]', v).forEach((r) => r.addEventListener("change", () => { guardarAjuste("tema", r.value); aplicarAjustes(); }));
  $("#aj-mov", v).addEventListener("change", (e) => { guardarAjuste("movimiento", e.target.checked ? "reducido" : ""); aplicarAjustes(); });
}

// ---------- Navegación ----------

const VISTAS = {
  tablero: ["Tablero", vistaTablero],
  resumen: ["Resumen", vistaResumen],
  movimientos: ["Movimientos", vistaMovimientos],
  deudas: ["Deudas", vistaDeudas],
  tarjetas: ["Tarjetas de crédito", vistaTarjetas],
  catalogos: ["Catálogos", vistaCatalogos],
  datos: ["Importar y exportar", vistaDatos],
  ajustes: ["Ajustes", vistaAjustes],
};

async function recargar() {
  [estado.config, estado.cat] = await Promise.all([api("config"), api("catalogos")]);
  await render(false);
}

async function render(enfocar = false) {
  const [titulo, fn] = VISTAS[estado.vista] ?? VISTAS.tablero;
  ocultarTip();
  document.getElementById("calendario")?.remove();
  $("#titulo").textContent = titulo;
  document.title = `${titulo} · Sueldapp`;
  $$(".menu a").forEach((a) => (a.dataset.vista === estado.vista ? a.setAttribute("aria-current", "page") : a.removeAttribute("aria-current")));
  const v = $("#vista");
  try {
    await fn(v);
  } catch (e) {
    v.innerHTML = `<section class="panel"><p class="error" role="alert">${esc(e.message)}</p></section>`;
  }
  $$("[data-ir]", v).forEach((b) => b.addEventListener("click", () => (location.hash = b.dataset.ir)));
  if (enfocar) $("#principal").focus();
}

function irA(hash) {
  const vista = hash.replace("#", "") || "tablero";
  estado.vista = VISTAS[vista] ? vista : "tablero";
  render(true);
}

window.addEventListener("hashchange", () => irA(location.hash));
document.addEventListener("keydown", (e) => {
  if (!e.altKey || $("#dialogo").open) return;
  const claves = Object.keys(VISTAS);
  const n = Number(e.key);
  if (n >= 1 && n <= claves.length) { e.preventDefault(); location.hash = claves[n - 1]; }
  if (estado.vista === "resumen" && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
    e.preventDefault();
    cambiarMes((e.key === "ArrowLeft" ? -1 : 1) * (estado.modoResumen === "mes" ? 1 : 12));
  }
});

(async function iniciar() {
  try {
    [estado.config, estado.cat] = await Promise.all([api("config"), api("catalogos")]);
  } catch (e) {
    $("#vista").innerHTML = `<section class="panel"><p class="error" role="alert">${esc(e.message)}</p></section>`;
    return;
  }
  aplicarAjustes();
  $("#demo-aviso").hidden = !estado.config.demo;
  if (estado.config.demo) { estado.anio = 2026; estado.mes = 10; }
  irA(location.hash);
  if (estado.config.capturas) setTimeout(capturarTodo, 800);
})();

// Solo desarrollo: recorre las pantallas en los tres modos y las captura (SUELDAPP_CAPTURAS).
async function capturarTodo() {
  const raiz = document.documentElement;
  const espera = (ms) => new Promise((r) => setTimeout(r, ms));
  const modos = [["claro", false], ["oscuro", false], ["claro", true], ["oscuro", true]];
  if (formatearEntrada("1234567,891") !== "1.234.567,89" || formatearEntrada("0012,5") !== "12,5" || formatearEntrada(",5") !== "0,5") {
    await api("capturar", { nombre: "prueba-fallo-mascara" });
  }
  raiz.dataset.movimiento = "reducido"; // sin animaciones: la captura no debe pillar un fotograma intermedio
  for (const [tema, reducida] of modos) {
    raiz.dataset.tema = tema;
    if (reducida) raiz.dataset.transparencia = "reducida"; else delete raiz.dataset.transparencia;
    const sufijo = tema + (reducida ? "-solido" : "");
    for (const vista of Object.keys(VISTAS)) {
      estado.vista = vista;
      estado.modoResumen = "mes";
      await render();
      $("#principal").scrollTop = 0;
      await espera(500);
      await api("capturar", { nombre: `${vista}-${sufijo}` });
      if (vista === "movimientos") {
        $("#principal").scrollTop = 99999;
        await espera(300);
        await api("capturar", { nombre: `movimientos-pie-${sufijo}` });
      }
      if (vista === "tablero") {
        $("#abrir-rango")?.click();
        await espera(300);
        await api("capturar", { nombre: `tablero-calendario-${sufijo}` });
        $("#abrir-rango")?.click();
        for (const [i, y] of [800, 1600].entries()) {
          $("#principal").scrollTop = y;
          await espera(300);
          await api("capturar", { nombre: `tablero-${i + 2}-${sufijo}` });
        }
      }
    }
    estado.vista = "resumen"; estado.modoResumen = "anio";
    await render(); await espera(400);
    await api("capturar", { nombre: `resumen-anio-${sufijo}` });
    estado.modoResumen = "mes"; await render();
    await editarGasto((await api("resumen_mes", { anio: estado.anio, mes: estado.mes })).gastos[0]);
    await espera(900);
    await api("capturar", { nombre: `dialogo-gasto-${sufijo}` });
    $("#dialogo").close();
    estado.vista = "deudas"; await render();
    $$("details", $("#vista")).slice(0, 1).forEach((d) => (d.open = true));
    await espera(300);
    await api("capturar", { nombre: `deudas-detalle-${sufijo}` });
  }
  // Prueba de humo de los comandos de edición (en la base de demostración).
  let resultado = "prueba-ok";
  try {
    const p1 = await api("buscar_gastos", { filtro: { limite: 10 } });
    const p2 = await api("buscar_gastos", { filtro: { limite: 10, cursor: p1.cursor_siguiente, direccion: "siguiente" } });
    const p1b = await api("buscar_gastos", { filtro: { limite: 10, cursor: p2.cursor_anterior, direccion: "anterior" } });
    if (p1.gastos.length !== 10 || p2.desde !== 11 || p1b.gastos.map((g) => g.id).join() !== p1.gastos.map((g) => g.id).join()) {
      throw new Error("paginacion");
    }
    const g = await api("guardar_gasto", { gasto: { id: null, anio: 2026, mes: 10, concepto: "Prueba", monto: "1.234,5",
      estado_id: estado.cat.estados[0].id, categoria_id: null, deuda_id: estado.cat.deudas[0]?.id ?? null, tarjeta_id: null, nota: "", fuera_de_bloque: false } });
    if (g.monto_texto !== "$ 1.234,50") throw new Error("monto " + g.monto_texto);
    const borrado = await api("borrar_gasto", { id: g.id });
    await api("restaurar_gasto", { gasto: borrado });
    await api("borrar_gasto", { id: g.id });
    const d = await api("borrar_deuda", { id: estado.cat.deudas[0].id });
    await api("restaurar_deuda", { borrado: d });
    const t = await api("guardar_tarjeta", { tarjeta: { id: null, nombre: "Prueba", cupo: "100.000", saldo: "", dia_corte: 5, dia_pago: null } });
    await api("borrar_tarjeta", { id: t.id });
    const cat = await api("guardar_item", { tipo: "categorias", item: { id: null, nombre: "Prueba" } });
    await api("borrar_item", { tipo: "categorias", id: cat.id, reasignarA: null });
    const m = await api("guardar_mes", { mes: { id: null, anio: 2030, mes: 1, cerrado: true } });
    const mc = await api("borrar_mes", { id: m.id });
    await api("restaurar_mes", { mes: mc });
    await api("borrar_mes", { id: m.id });
    try { await api("guardar_gasto", { gasto: { id: null, anio: 2026, mes: 10, concepto: "x", monto: "abc", estado_id: 1, categoria_id: null, deuda_id: null, tarjeta_id: null, nota: "", fuera_de_bloque: false } }); resultado = "prueba-fallo-validacion"; } catch { /* esperado */ }
  } catch (e) {
    resultado = "prueba-fallo-" + e.message.replace(/[^a-z0-9]+/gi, "-").slice(0, 60);
  }
  await api("capturar", { nombre: resultado });
  aviso("Capturas terminadas.");
}
