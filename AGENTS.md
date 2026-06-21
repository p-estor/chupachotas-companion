# AGENTS.md — chupachotas-companion

Hereda las reglas del AGENTS.md global (Ponytail). Esto añade contexto
específico de este repo.

**Project profile: ni "portfolio" ni "online tool" puro — es una app de
escritorio en fase PoC/alpha, sin usuarios reales todavía.** Trátalo como
moderate mode igualmente en todo lo que toque el lockfile de Riot, llamadas
HTTP al LCU, o el manejo de coordenadas del cursor (Win API): son
trust-boundary code aunque no haya "usuarios externos" en sentido estricto,
porque interactúan con un proceso del sistema y con datos no controlados
por nosotros.

## Qué es esto
App de escritorio (Tauri v2) que se conecta al cliente local de League of
Legends (LCU API) para mostrar recomendaciones de draft y un overlay HUD
sobre el juego. Todavía en fase de prueba de concepto — el propio
`package.json` y `Cargo.toml` siguen nombrados `tauri-overlay-poc`, no
`chupachotas-companion`. No asumas que esto es producción estable; es
software en development activo y experimental.

## Stack real (verificado en código)
- Tauri v2, Rust (edición 2021). `src-tauri/src/lib.rs` (~17.6K, el grueso
  de la lógica) + `main.rs` (entrypoint mínimo).
- Dependencias clave en `Cargo.toml`: `reqwest` 0.12 (cliente HTTP async),
  `tokio` 1.43 con feature `full`, `winapi` 0.3.9 (solo en target Windows,
  features `winuser`/`windef`).
- Frontend: HTML/CSS/JS vanilla, sin framework ni build step de JS más allá
  de lo que Tauri necesita. Dos superficies separadas:
  - `src/index.html` + `src/styles.css` + `src/main.js` → ventana principal.
  - `src/overlay.html` + `src/overlay.css` + `src/overlay.js` → ventana de
    overlay independiente.

## Puntos sensibles del Rust — no tocar sin entender el porqué
- `LOCKFILE_PATH` está hardcodeado a la ruta de Windows
  (`C:\Riot Games\League of Legends\lockfile`). Es así porque el LCU
  siempre coloca el lockfile ahí en instalaciones estándar de Windows — no
  es un descuido, es la única ruta válida en ese SO. No lo conviertas en
  configurable salvo que se pida soporte multi-SO explícitamente.
- `.danger_accept_invalid_certs(true)` en la configuración del cliente
  `reqwest` (línea ~265 de `lib.rs`) es **necesario**, no un error de
  seguridad: el LCU sirve HTTPS local con un certificado autofirmado
  específico de cada sesión del cliente de LoL, no hay forma de validarlo
  contra una CA pública. No lo quites. Pero tampoco repliques este patrón
  para ninguna llamada a una API externa real (Riot Data Dragon, Riot API
  pública, etc.) — ahí sí debe validarse el certificado normalmente.
- `GetCursorPos` (winapi) se usa para leer la posición física del cursor y
  habilitar click-through selectivo en la ventana de overlay. Es lógica
  Windows-only por diseño (la app en su estado actual apunta a Windows,
  donde vive el cliente de LoL). No abstraigas esto a multiplataforma sin
  que se pida.

## Diseño / CSS — el caso más urgente de unificación
- `src/styles.css` define `:root` con tokens (`--bg-primary`,
  `--accent-cyan: #00f0ff`, etc.) usados en `index.html`.
- `src/overlay.css` **no importa `styles.css` y no tiene su propio
  `:root`** — repite los mismos hex a mano (`#00f0ff`, `#8a2be2`,
  `#94a3b8`...) directamente en cada regla. Si cambias un color en
  `styles.css`, el overlay no se entera; hay que tocar los dos sitios.
  Esto es la inconsistencia más clara de los tres repos — ver
  `DESIGN_SYSTEM.md` sección 4.3. Si te piden "unificar colores del
  companion", el primer paso casi seguro es resolver esto: o bien inyectar
  las mismas custom properties también en `overlay.html` (vía un tercer
  archivo CSS compartido que ambos importen), o aceptar conscientemente
  que el overlay vive desincronizado y documentarlo.
- Paleta: coincide exactamente con `chupachotas-tracker`
  (`#00f0ff`/`#8a2be2`/`#00ff87`/`#ff4655`), pero diverge de
  `soloq-challenge`. Solo dos niveles de texto (`--text-main`/
  `--text-muted`) frente a los tres niveles de tracker/soloq — si portas
  un componente desde alguno de esos dos repos, puede que referencie un
  tercer nivel que aquí no existe.
- Tipografía Outfit cargada con pesos `300;400;600;800` — **falta 500 y
  700** respecto a tracker/soloq, que cargan hasta 700. Si copias CSS de
  otro repo que use `font-weight: 500` o `700`, añade esos pesos al
  `<link>` de Google Fonts en ambos HTML, o el navegador hará fallback
  silencioso a un peso distinto del diseñado.

## Distribución
Sin pipeline de build/release visible en este repo (no hay
`.github/workflows`, no hay configuración de `tauri-plugin-updater`). La
distribución a usuarios finales como binario todavía no está resuelta —
no asumas que existe un instalador o canal de auto-actualización.
