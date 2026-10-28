// Tablero: estadísticas y gráficos SVG. Los números llegan calculados desde Rust; aquí solo se
// convierten a Number para ubicar las marcas en pantalla (geometría), nunca para sumar dinero.
"use strict";

const NS = "http://www.w3.org/2000/svg";
const compacto = new Intl.NumberFormat("es-CO", { notation: "compact", maximumFractionDigits: 1 });
const ejeMonto = (n) => (n < 0 ? "-$ " : "$ ") + compacto.format(Math.abs(n));

/** Escala "bonita" para el eje: devuelve [min, max, paso]. */
function escala(min, max, marcas = 4) {
  if (min === max) { max = min + 1; }
  const rango = max - min;
  const bruto = rango / marcas;
  const mag = 10 ** Math.floor(Math.log10(bruto));
  const paso = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((p) => p >= bruto) ?? bruto;
  return [Math.floor(min / paso) * paso, Math.ceil(max / paso) * paso, paso];
}

function el(nombre, attrs = {}, padre = null) {
  const e = document.createElementNS(NS, nombre);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  if (padre) padre.append(e);
  return e;
}

/** Barra con el extremo de datos redondeado (4px) y anclada a la base. */
function barra(x, y0, y1, w, padre, clase) {
  const r = Math.min(4, w / 2, Math.abs(y1 - y0));
  const arriba = y1 < y0;
  const yt = Math.min(y0, y1), h = Math.abs(y1 - y0);
  let d;
  if (h < 0.5) return;
  if (arriba) {
    d = `M${x},${y0} V${yt + r} Q${x},${yt} ${x + r},${yt} H${x + w - r} Q${x + w},${yt} ${x + w},${yt + r} V${y0} Z`;
  } else {
    const yb = yt + h;
    d = `M${x},${y0} V${yb - r} Q${x},${yb} ${x + r},${yb} H${x + w - r} Q${x + w},${yb} ${x + w},${yb - r} V${y0} Z`;
  }
  el("path", { d, class: clase }, padre);
}

const tip = () => document.getElementById("tip");
function mostrarTip(ev, html) {
  const t = tip();
  t.innerHTML = html;
  t.hidden = false;
  const r = t.getBoundingClientRect();
  let x = ev.clientX + 14, y = ev.clientY + 14;
  if (x + r.width > innerWidth - 8) x = ev.clientX - r.width - 14;
  if (y + r.height > innerHeight - 8) y = ev.clientY - r.height - 14;
  t.style.left = `${x}px`;
  t.style.top = `${y}px`;
}
const ocultarTip = () => { tip().hidden = true; };

/** Ejes y rejilla comunes. Devuelve funciones de escala. */
function lienzo(cont, n, vmin, vmax) {
  const ancho = Math.max(320, cont.clientWidth), alto = 260;
  const m = { izq: 64, der: 12, arr: 12, aba: 34 };
  const svg = el("svg", { width: ancho, height: alto, viewBox: `0 0 ${ancho} ${alto}`, role: "img" });
  const [lo, hi, paso] = escala(Math.min(0, vmin), Math.max(0, vmax));
  const y = (v) => m.arr + (hi - v) / (hi - lo) * (alto - m.arr - m.aba);
  const banda = (ancho - m.izq - m.der) / n;
  const x = (i) => m.izq + i * banda;
  const g = el("g", { class: "eje" }, svg);
  for (let v = lo; v <= hi + paso / 2; v += paso) {
    el("line", { x1: m.izq, x2: ancho - m.der, y1: y(v), y2: y(v), class: v === 0 ? "base" : "rejilla" }, g);
    const t = el("text", { x: m.izq - 8, y: y(v) + 4, "text-anchor": "end" }, g);
    t.textContent = ejeMonto(v);
  }
  return { svg, y, x, banda, ancho, alto, m };
}

function etiquetasX(l, meses) {
  const cada = Math.ceil(meses.length / Math.max(1, Math.floor((l.ancho - l.m.izq) / 64)));
  meses.forEach((mes, i) => {
    if (i % cada) return;
    const t = el("text", { x: l.x(i) + l.banda / 2, y: l.alto - 12, "text-anchor": "middle", class: "eje-x" }, l.svg);
    t.textContent = mes.corto;
  });
}

/** Zona de hover por mes (más grande que la marca). */
function zonas(l, meses, html) {
  meses.forEach((mes, i) => {
    const z = el("rect", { x: l.x(i), y: l.m.arr, width: l.banda, height: l.alto - l.m.arr - l.m.aba, class: "zona" }, l.svg);
    z.addEventListener("mousemove", (e) => { mostrarTip(e, html(mes)); z.classList.add("activa"); });
    z.addEventListener("mouseleave", () => { ocultarTip(); z.classList.remove("activa"); });
  });
}

function graficoIngresosGastos(cont, meses) {
  const vals = meses.flatMap((m) => [Number(m.ingreso.valor), Number(m.gasto.valor)]);
  const l = lienzo(cont, meses.length, Math.min(...vals), Math.max(...vals));
  l.svg.setAttribute("aria-label", "Ingresos y gastos por mes. Los valores están en la tabla de datos.");
  zonas(l, meses, (m) => `<b>${esc(m.nombre)}</b><br><span class="punto s1"></span>Ingresos ${esc(m.ingreso.texto)}<br><span class="punto s2"></span>Gastos ${esc(m.gasto.texto)}`);
  const w = Math.max(2, Math.min(18, (l.banda - 6) / 2 - 1));
  meses.forEach((m, i) => {
    const c = l.x(i) + l.banda / 2;
    barra(c - w - 1, l.y(0), l.y(Number(m.ingreso.valor)), w, l.svg, "serie-1");
    barra(c + 1, l.y(0), l.y(Number(m.gasto.valor)), w, l.svg, "serie-2");
  });
  etiquetasX(l, meses);
  cont.replaceChildren(l.svg);
}

function graficoBalance(cont, meses) {
  const vals = meses.map((m) => Number(m.balance.valor));
  const l = lienzo(cont, meses.length, Math.min(...vals), Math.max(...vals));
  l.svg.setAttribute("aria-label", "Lo que sobra o falta cada mes. Los valores están en la tabla de datos.");
  zonas(l, meses, (m) => `<b>${esc(m.nombre)}</b><br>${m.balance.negativo ? "Faltó" : "Sobró"} ${esc(m.balance.texto.replace("-", ""))}${m.tasa_ahorro ? `<br>${esc(m.tasa_ahorro.texto)} del ingreso` : ""}`);
  const w = Math.max(2, Math.min(28, l.banda - 6));
  meses.forEach((m, i) => {
    const v = Number(m.balance.valor);
    barra(l.x(i) + (l.banda - w) / 2, l.y(0), l.y(v), w, l.svg, v < 0 ? "polo-neg" : "polo-pos");
  });
  etiquetasX(l, meses);
  cont.replaceChildren(l.svg);
}

function graficoAcumulado(cont, meses) {
  const vals = meses.map((m) => Number(m.acumulado.valor));
  const l = lienzo(cont, meses.length, Math.min(...vals), Math.max(...vals));
  l.svg.setAttribute("aria-label", "Saldo acumulado mes a mes. Los valores están en la tabla de datos.");
  const pts = meses.map((m, i) => [l.x(i) + l.banda / 2, l.y(Number(m.acumulado.valor))]);
  el("path", { d: `M${pts[0][0]},${l.y(0)} ` + pts.map((p) => `L${p[0]},${p[1]}`).join(" ") + ` L${pts.at(-1)[0]},${l.y(0)} Z`, class: "area-1" }, l.svg);
  el("path", { d: "M" + pts.map((p) => p.join(",")).join(" L"), class: "linea-1" }, l.svg);
  const cruz = el("line", { y1: l.m.arr, y2: l.alto - l.m.aba, class: "cruz", visibility: "hidden" }, l.svg);
  const punto = el("circle", { r: 5, class: "punto-1", visibility: "hidden" }, l.svg);
  meses.forEach((m, i) => {
    const z = el("rect", { x: l.x(i), y: l.m.arr, width: l.banda, height: l.alto - l.m.arr - l.m.aba, class: "zona sin-fondo" }, l.svg);
    z.addEventListener("mousemove", (e) => {
      for (const [a, v] of [["x1", pts[i][0]], ["x2", pts[i][0]]]) cruz.setAttribute(a, v);
      punto.setAttribute("cx", pts[i][0]); punto.setAttribute("cy", pts[i][1]);
      cruz.setAttribute("visibility", "visible"); punto.setAttribute("visibility", "visible");
      mostrarTip(e, `<b>${esc(m.nombre)}</b><br>Acumulado ${esc(m.acumulado.texto)}`);
    });
    z.addEventListener("mouseleave", () => { ocultarTip(); cruz.setAttribute("visibility", "hidden"); punto.setAttribute("visibility", "hidden"); });
  });
  etiquetasX(l, meses);
  cont.replaceChildren(l.svg);
}

/** Barras horizontales con etiqueta y valor directos (un solo color: identidad por texto). */
function barrasHorizontales(grupos, { clase = "serie-1", detalle = (g) => g.porcentaje?.texto ?? "" } = {}) {
  if (!grupos.length) return '<p class="ayuda">Sin datos en este periodo.</p>';
  const max = Math.max(...grupos.map((g) => Number(g.monto.valor)), 1);
  return `<ul class="barras-h">${grupos.map((g) => {
    const p = Math.max(0, Math.round((Number(g.monto.valor) / max) * 1000) / 10);
    return `<li><div class="bh-cab"><span class="bh-nombre">${esc(g.nombre)}</span><span class="bh-valor">${esc(g.monto.texto)} <small>${esc(detalle(g))}</small></span></div>
      <svg class="bh-pista" viewBox="0 0 100 8" preserveAspectRatio="none" aria-hidden="true"><rect x="0" y="0" width="${p}" height="8" rx="3" class="${clase}"/></svg></li>`;
  }).join("")}</ul>`;
}

function kpi(etiqueta, valor, nota = "", negativo = false) {
  return `<div class="kpi"><dt>${esc(etiqueta)}</dt><dd class="${negativo ? "negativo" : ""}">${esc(valor)}</dd>${nota ? `<p class="ayuda">${nota}</p>` : ""}</div>`;
}

// ---------- Selector de periodo (meses) ----------

const CORTOS = ["Ene", "Feb", "Mar", "Abr", "May", "Jun", "Jul", "Ago", "Sep", "Oct", "Nov", "Dic"];
const clavePer = ([a, m]) => a * 12 + (m - 1);
const dePer = (k) => [Math.floor(k / 12), (k % 12) + 1];
const textoPer = (p) => (p ? `${CORTOS[p[1] - 1]} ${p[0]}` : "");

function presets(min, max) {
  const a = hoy.getFullYear(), m = hoy.getMonth() + 1, ahora = [a, m];
  const atras = (n) => dePer(clavePer(ahora) - (n - 1));
  return [
    ["Hasta hoy", min, ahora],
    ["Este mes", ahora, ahora],
    ["Últimos 3 meses", atras(3), ahora],
    ["Últimos 6 meses", atras(6), ahora],
    ["Últimos 12 meses", atras(12), ahora],
    ["Este año", [a, 1], [a, 12]],
    ["Año pasado", [a - 1, 1], [a - 1, 12]],
    ["Todo (con proyección)", min, max],
  ];
}

/** Rango por defecto: desde el primer mes con datos hasta el mes actual. */
function rangoInicial(meses) {
  const min = meses[0] ? [meses[0].anio, meses[0].mes] : [hoy.getFullYear(), 1];
  return { desde: min, hasta: [hoy.getFullYear(), hoy.getMonth() + 1], nombre: "Hasta hoy" };
}

function barraFiltros(meses) {
  const r = estado.rangoTablero;
  const anios = [...new Set(meses.map((m) => m.anio))];
  const esAnio = (a) => clavePer(r.desde) === clavePer([a, 1]) && clavePer(r.hasta) === clavePer([a, 12]);
  return `<section class="panel filtros-tablero" aria-label="Periodo del tablero">
    <div class="selector-rango">
      <button type="button" class="boton-rango" id="abrir-rango" aria-haspopup="dialog" aria-expanded="false">
        <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3.5" y="5" width="17" height="15" rx="3"/><path d="M3.5 10h17M8 3v4m8-4v4"/></svg>
        <span><span class="rango-nombre">${esc(r.nombre ?? "Periodo personalizado")}</span>
        <b>${esc(textoPer(r.desde))} – ${esc(textoPer(r.hasta))}</b></span>
        <svg class="flecha" viewBox="0 0 24 24" aria-hidden="true"><path d="m7 10 5 5 5-5"/></svg>
      </button>
    </div>
    <div class="anios" role="group" aria-label="Año completo">
      <span class="etiqueta">Años</span>
      ${anios.map((a) => `<button type="button" class="chip-anio" data-anio="${a}" aria-pressed="${esAnio(a)}">${a}</button>`).join("")}
    </div>
  </section>`;
}

function activarFiltros(v, meses) {
  const min = meses[0] ? [meses[0].anio, meses[0].mes] : [hoy.getFullYear(), 1];
  const max = meses.at(-1) ? [meses.at(-1).anio, meses.at(-1).mes] : [hoy.getFullYear(), 12];
  const conDatos = new Set(meses.map((m) => clavePer([m.anio, m.mes])));
  const boton = $("#abrir-rango", v);
  document.getElementById("calendario")?.remove();
  // Va en <body> con posición fija: así queda por encima de todos los paneles de vidrio.
  const cal = document.createElement("div");
  cal.className = "calendario";
  cal.id = "calendario";
  cal.setAttribute("role", "dialog");
  cal.setAttribute("aria-label", "Elegir periodo");
  let anioVista, inicio, fin, previa;

  $$("[data-anio]", v).forEach((b) => b.addEventListener("click", () => {
    const a = Number(b.dataset.anio);
    estado.rangoTablero = { desde: [a, 1], hasta: [a, 12], nombre: `Año ${a}` };
    render();
  }));

  const pintar = () => {
    const a0 = Math.min(...[inicio, fin ?? previa].filter(Boolean).map(clavePer));
    const a1 = Math.max(...[inicio, fin ?? previa].filter(Boolean).map(clavePer));
    cal.innerHTML = `
      <div class="cal-presets" role="group" aria-label="Accesos rápidos">${presets(min, max).map(([n, d, h], i) =>
        `<button type="button" data-preset="${i}" class="${estado.rangoTablero.nombre === n ? "activo" : ""}">${esc(n)}</button>`).join("")}</div>
      <div class="cal-cuerpo">
        <div class="cal-cabecera">
          <button type="button" class="icono" data-anio-vista="-1" aria-label="Año anterior">${ICONOS.anterior}</button>
          <strong aria-live="polite">${anioVista}</strong>
          <button type="button" class="icono" data-anio-vista="1" aria-label="Año siguiente">${ICONOS.siguiente}</button>
        </div>
        <div class="cal-meses" role="grid" aria-label="Meses de ${anioVista}">${CORTOS.map((n, i) => {
          const k = clavePer([anioVista, i + 1]);
          const cls = [
            k === clavePer(inicio) ? "extremo inicio" : "",
            fin && k === clavePer(fin) ? "extremo fin" : "",
            k > a0 && k < a1 ? "dentro" : "",
            conDatos.has(k) ? "" : "sin-datos",
            k === clavePer([hoy.getFullYear(), hoy.getMonth() + 1]) ? "hoy" : "",
          ].join(" ");
          return `<button type="button" role="gridcell" data-mes="${i + 1}" class="${cls}" aria-pressed="${k >= a0 && k <= a1}"
            aria-label="${MESES[i]} ${anioVista}${conDatos.has(k) ? "" : ", sin datos"}">${n}</button>`;
        }).join("")}</div>
        <p class="cal-ayuda">${fin ? `${esc(textoPer(inicio))} – ${esc(textoPer(fin))}` : `Desde ${esc(textoPer(inicio))}: elige el mes final`}</p>
        <div class="cal-pie"><button type="button" data-cal="cancelar">Cancelar</button>
          <button type="button" class="primario" data-cal="aplicar" ${fin ? "" : "disabled"}>Aplicar</button></div>
      </div>`;
    $$("[data-preset]", cal).forEach((b) => b.addEventListener("click", () => {
      const [n, d, h] = presets(min, max)[Number(b.dataset.preset)];
      estado.rangoTablero = { desde: d, hasta: h, nombre: n };
      cerrar(); render();
    }));
    $$("[data-anio-vista]", cal).forEach((b) => b.addEventListener("click", () => { anioVista += Number(b.dataset.anioVista); pintar(); $(`[data-anio-vista="${b.dataset.anioVista}"]`, cal).focus(); }));
    $$("[data-mes]", cal).forEach((b) => {
      const p = [anioVista, Number(b.dataset.mes)];
      b.addEventListener("click", () => {
        if (!inicio || fin) { inicio = p; fin = null; }
        else if (clavePer(p) < clavePer(inicio)) { fin = inicio; inicio = p; }
        else fin = p;
        previa = null;
        pintar();
        $(`[data-mes="${p[1]}"]`, cal)?.focus();
      });
      b.addEventListener("mouseenter", () => { if (inicio && !fin) { previa = p; pintar(); } });
    });
    $('[data-cal="cancelar"]', cal).addEventListener("click", () => { cerrar(); boton.focus(); });
    $('[data-cal="aplicar"]', cal).addEventListener("click", () => {
      estado.rangoTablero = { desde: inicio, hasta: fin, nombre: null };
      cerrar(); render();
    });
  };

  const fuera = (e) => { if (!cal.contains(e.target) && !boton.contains(e.target)) cerrar(); };
  const escape = (e) => { if (e.key === "Escape") { cerrar(); boton.focus(); } };
  function cerrar() {
    cal.remove();
    $("#principal").removeEventListener("scroll", cerrar);
    boton.setAttribute("aria-expanded", "false");
    document.removeEventListener("mousedown", fuera);
    document.removeEventListener("keydown", escape);
  }
  boton.addEventListener("click", () => {
    if (cal.isConnected) return cerrar();
    inicio = estado.rangoTablero.desde; fin = estado.rangoTablero.hasta; previa = null;
    anioVista = fin[0];
    pintar();
    const r = boton.getBoundingClientRect();
    cal.style.left = `${Math.max(8, Math.min(r.left, innerWidth - 500))}px`;
    cal.style.top = `${r.bottom + 10}px`;
    document.body.append(cal);
    $("#principal").addEventListener("scroll", cerrar);
    boton.setAttribute("aria-expanded", "true");
    document.addEventListener("mousedown", fuera);
    document.addEventListener("keydown", escape);
    $(".cal-meses button.extremo", cal)?.focus();
  });
}

let ultimoTablero = null;

async function vistaTablero(v) {
  acciones("");
  const meses = await api("listar_meses");
  estado.rangoTablero ??= rangoInicial(meses);
  if (estado.config?.vacia) { v.innerHTML = vistaVacia(); return; }
  const t = await api("tablero", { desde: estado.rangoTablero.desde, hasta: estado.rangoTablero.hasta });
  ultimoTablero = t;
  const k = t.kpis;
  const filtros = barraFiltros(meses);
  if (!t.meses.length) {
    v.innerHTML = filtros + '<section class="panel vacio"><h2>No hay meses en este periodo</h2><p>Elige otro periodo arriba.</p></section>';
    activarFiltros(v, meses);
    return;
  }
  v.innerHTML = filtros + `
    <section class="panel heroe" aria-labelledby="t-balance">
      <div>
        <div class="etiqueta" id="t-balance">${k.balance.negativo ? "Te faltó en el periodo" : "Te sobró en el periodo"}</div>
        <div class="grande ${k.balance.negativo ? "negativo" : ""}">${esc(k.balance.texto)}</div>
        <p class="ayuda">${esc(t.desde)} – ${esc(t.hasta)} · ${k.meses} meses.
          ${k.tasa_ahorro ? `Ahorraste el <b>${esc(k.tasa_ahorro.texto)}</b> de lo que ingresó.` : ""}</p>
      </div>
      <dl class="cifras">
        <div><dt>Ingresos</dt><dd>${cifra(k.ingreso)}</dd></div>
        <div><dt>Gastos</dt><dd>${cifra(k.gasto)}</dd></div>
        <div><dt>Meses con sobrante</dt><dd>${k.meses_con_sobrante} <small class="ayuda">${esc(k.sobrante.texto)}</small></dd></div>
        <div><dt>Meses con faltante</dt><dd class="${k.meses_con_faltante ? "negativo" : ""}">${k.meses_con_faltante} <small class="ayuda">${esc(k.faltante.texto)}</small></dd></div>
      </dl>
    </section>
    <section class="panel"><h2>Promedios y extremos</h2>
      <dl class="kpis">
        ${kpi("Ingreso promedio al mes", k.promedio_ingreso.texto)}
        ${kpi("Gasto promedio al mes", k.promedio_gasto.texto)}
        ${kpi("Sobra o falta en promedio", k.promedio_balance.texto, "", k.promedio_balance.negativo)}
        ${kpi("Por pagar en el periodo", k.por_pagar.texto)}
        ${k.mejor_mes ? kpi("Mejor mes", k.mejor_mes.nombre, `Sobró <span class="cifra">${esc(k.mejor_mes.monto.texto)}</span>`) : ""}
        ${k.peor_mes ? kpi("Mes más ajustado", k.peor_mes.nombre, `${k.peor_mes.monto.negativo ? "Faltó" : "Sobró"} <span class="cifra">${esc(k.peor_mes.monto.texto.replace("-", ""))}</span>`) : ""}
        ${k.mes_mayor_gasto ? kpi("Mes con más gasto", k.mes_mayor_gasto.nombre, esc(k.mes_mayor_gasto.monto.texto)) : ""}
        ${kpi("Deuda pendiente total", t.deuda_pendiente.texto, `Ya pagado en deudas: <span class="cifra">${esc(t.deuda_pagada.texto)}</span>`)}
      </dl>
    </section>
    <section class="panel" aria-labelledby="g-ing"><div class="cabecera-panel"><h2 id="g-ing">Ingresos y gastos por mes</h2>
      <ul class="leyenda"><li><span class="punto s1"></span>Ingresos</li><li><span class="punto s2"></span>Gastos</li></ul></div>
      <div class="grafico" data-grafico="ingresos"></div></section>
    <div class="columnas iguales">
      <section class="panel" aria-labelledby="g-bal"><div class="cabecera-panel"><h2 id="g-bal">Lo que sobra o falta cada mes</h2>
        <ul class="leyenda"><li><span class="punto pos"></span>Sobra</li><li><span class="punto neg"></span>Falta</li></ul></div>
        <div class="grafico" data-grafico="balance"></div></section>
      <section class="panel" aria-labelledby="g-acu"><div class="cabecera-panel"><h2 id="g-acu">Saldo acumulado</h2>
        <p class="ayuda">Suma de lo que sobra y falta, mes tras mes.</p></div>
        <div class="grafico" data-grafico="acumulado"></div></section>
    </div>
    <div class="columnas tres">
      <section class="panel"><h2>¿En qué se va el dinero?</h2>${barrasHorizontales(t.por_destino)}</section>
      <section class="panel"><h2>Gasto por categoría</h2>${barrasHorizontales(t.por_categoria)}</section>
      <section class="panel"><h2>Estado de los pagos</h2>${barrasHorizontales(t.por_estado)}</section>
    </div>
    <div class="columnas">
      <section class="panel"><h2>Conceptos con más gasto</h2>${barrasHorizontales(t.top_conceptos, { detalle: (g) => `${g.veces} ${g.veces === 1 ? "vez" : "veces"}` })}</section>
      <section class="panel"><h2>Deudas pendientes</h2>
        ${t.deudas.length ? `<ul class="lista">${t.deudas.map((d) => `<li><div><div class="concepto">${esc(d.nombre)}</div>
          ${d.progreso != null ? `<progress class="progreso" max="100" value="${d.progreso}" aria-label="Progreso de ${esc(d.nombre)}"></progress><div class="sub">${String(d.progreso).replace(".", ",")} % pagado</div>` : ""}</div>
          <span class="monto cifra">${esc(d.pendiente.texto)}</span><span></span></li>`).join("")}</ul>` : '<p class="ayuda">No tienes deudas pendientes.</p>'}
      </section>
    </div>
    <div class="columnas">
      <section class="panel"><h2>Gastos más grandes</h2>
        <ul class="lista">${t.mayores_gastos.map((g) => `<li><div><div class="concepto">${esc(g.concepto)}</div><div class="sub">${esc(g.periodo)}</div></div><span class="monto cifra">${esc(g.monto_texto)}</span><span></span></li>`).join("")}</ul></section>
      <section class="panel"><h2>Ingresos por fuente</h2>${barrasHorizontales(t.por_fuente)}</section>
    </div>
    <section class="panel"><details><summary>Ver los datos de los gráficos en tabla</summary>
      <div class="tabla-scroll"><table><thead><tr><th>Mes</th><th class="monto">Ingresos</th><th class="monto">Gastos</th><th class="monto">Pagado</th><th class="monto">Sobra / falta</th><th class="monto">Acumulado</th><th class="monto">Ahorro</th></tr></thead>
      <tbody>${t.meses.map((m) => `<tr><td>${esc(m.nombre)}</td><td class="monto">${cifra(m.ingreso)}</td><td class="monto">${cifra(m.gasto)}</td><td class="monto">${cifra(m.pagado)}</td><td class="monto">${cifra(m.balance)}</td><td class="monto">${cifra(m.acumulado)}</td><td class="monto">${esc(m.tasa_ahorro?.texto ?? "—")}</td></tr>`).join("")}</tbody></table></div>
    </details></section>`;
  activarFiltros(v, meses);
  dibujarGraficos();
}

function dibujarGraficos() {
  const t = ultimoTablero;
  if (!t || estado.vista !== "tablero") return;
  const g = (k) => document.querySelector(`[data-grafico="${k}"]`);
  if (g("ingresos")) graficoIngresosGastos(g("ingresos"), t.meses);
  if (g("balance")) graficoBalance(g("balance"), t.meses);
  if (g("acumulado")) graficoAcumulado(g("acumulado"), t.meses);
}

let esperaRedimension;
new ResizeObserver(() => { clearTimeout(esperaRedimension); esperaRedimension = setTimeout(dibujarGraficos, 120); })
  .observe(document.getElementById("vista"));
