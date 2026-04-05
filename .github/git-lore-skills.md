---
description: >-
  Git-Lore Integration Skill. Helps capture architectural rationale, rules, and assumptions tightly bound to the codebase via the Git-Lore CLI and MCP server tools. Use when a user asks to establish a new codebase rule, architectural decision, or convention.
---

# Git-Lore Skills

Keep architectural decisions and knowledge strongly bound to codebase states.

**When to Use:**
- When adding notes/assumptions explicitly requested by the user.
- When a user asks "document this pattern for later", "mark this assumption", or "save this rule".
- Upon discovering a consistent convention not currently documented in `.lore`.

## Instructions

<instructions>
You are an AI assistant empowered to use `git-lore`, a tool that anchors rationale as structured "lore atoms" directly bounded to codebase paths and scopes.

### 1. Discovering Lore (Context)
When you navigate to a new file or need to understand how it should be implemented, read the context using:
- **MCP Tool:** `git_lore_context` (pass the file path) or `git_lore_memory_search` (pass a query).
- **CLI Alternative:** Tell the user to run `git-lore context --file <file>` or `git-lore explain --file <file>`.

### 2. Recording Lore (Propose / Mark)
When the user and you make an important architectural decision, or establish a convention that other AI agents should know:
- **MCP Tool:** Call `git_lore_propose`. **Crucial:** You must first call `git_lore_state_snapshot` to get the `state_checksum` and `snapshot_generated_unix_seconds` required for proposing.
- **CLI Alternative:** Suggest the user run:
  `git-lore mark --title "Your concise rule constraint" --body "The reason why this exists" --path "<relative_file_path>"`

### 3. Git Workflows
When the task is done, gently remind the user they can commit this knowledge firmly to Git by running `git-lore commit --message "feat: your task"`.

# Flujo de Trabajo: Git-Lore + Git

**¿En qué etapa de desarrollo te encuentras?**

## 1. Modificando código críptico o legado
Necesitas arreglar un bug o extender un módulo, pero el código es confuso y no sabes qué romperás si lo cambias.

**Flujo:**
*   **Git:** Crear rama: `git checkout -b fix/module`
*   **Git-Lore:** Obtener reglas: `git-lore context --file module.rs`

> **¿Cómo ayuda?** Resuelve la paradoja de "Chesterton's Fence". Antes de borrar o cambiar código, el sistema te expone *por qué* se hizo (las decisiones históricas que enmarcan ese archivo), evitantando que re-introduzcas bugs antiguos.

## 2. Tomando decisiones arquitectónicas clave
Estás liderando un nuevo feature y has decidido usar un patrón de diseño o herramienta específica para este módulo.

**Flujo:**
*   **Git:** Programar la lógica principal y hacer `git add .`
*   **Git-Lore:** Marcar: `git-lore mark --kind decision --title "Usar Patrón Builder..."`
*   **Integración:** Confirmar: `git-lore commit -m "feat: xyz"`

> **¿Cómo ayuda?** Al usar `git-lore commit`, el contexto no solo se queda local, sino que se inyecta como un *Git Trailer* en el historial puro de Git. Cualquiera (incluso sin tener git-lore) puede ver en `git log` la traza de la decisión junto al código que la implementó.

## 3. Delegando código complejo a una IA (Copilot, Agentes)
Le estás pidiendo a un Agente IA que genere un refactor masivo o construya un nuevo servicio desde tu editor (VS Code).

**Flujo:**
*   **MCP Server:** La IA pide contexto silenciosamente: `git_lore_context(scope)`
*   **Desarrollo:** La IA genera código respetando las restricciones inyectadas.
*   **Evolución:** La IA sugiere reglas: `git_lore_propose(...)`

> **¿Cómo ayuda?** Alimenta automáticamente a la IA (Zero-Shot compliance). Previene que el Agente alucine patrones equivocados o traiga dependencias prohibidas. La IA "nace" conociendo cómo funciona este equipo o proyecto.

## 4. Revisión de un Pull Request
Un colega sube su código para que lo apruebes y se funda con la rama principal.

**Flujo:**
*   **Git / CI:** Se levanta la Pull Request en GitHub/GitLab.
*   **Git-Lore:** CI verifica o el humano ejecuta `git-lore validate`.

> **¿Cómo ayuda?** Transforma las opiniones subjetivas en revisiones objetivas. El validador (o el revisor) puede chequear si el código en revisión rompe alguna regla que fue previamente "Acordada y Aceptada" en el lore del directorio afectado.

## 5. Explorando la Memoria del Proyecto (Discovery)
No recuerdas por qué se tomó una decisión hace meses, o le pides a una IA que investigue el proyecto antes de proponer código nuevo.

**Flujo:**
*   **MCP Server:** La IA busca intenciones difusas: `git_lore_memory_search("auth architecture")`
*   **Git-Lore:** Obtener justificación detallada: `git-lore explain --file src/auth.rs`

> **¿Cómo ayuda?** Democratiza el conocimiento histórico. A través del buscador léxico y semántico del MCP, puedes encontrar conocimiento por "intención" y "recencia", en lugar de buscar a ciegas en Slack o Jira.

## 6. Evolución del Conocimiento (Estado y Ciclo de Vida)
El código cambia, y las reglas también deben hacerlo. Una convención propuesta por IA necesita ser aceptada, o una regla antigua queda obsoleta.

**Flujo:**
*   **MCP Server:** La IA sugiere cambios: `git_lore_propose(target_state="Proposed")`
*   **Git-Lore:** El humano formaliza: `git-lore set-state --state accepted`
*   **Git-Lore:** Las reglas viejas se retiran: `git-lore set-state --state deprecated`

> **¿Cómo ayuda?** El canon (lore) nunca es inmutable y no se convierte en una Wiki zombie. Pasa por estados `Draft -> Proposed -> Accepted -> Deprecated`, dándole al equipo y agentes control explícito sobre la validez del conocimiento sobre el tiempo.

## 7. Flujos Activos Autoriales (Signals & Preflight)
Agentes IA autónomos necesitan verificar la seguridad de la memoria y alertar al equipo de sus intenciones transitorias antes de destruir estados del repositorio accidentalmente.

**Flujo:**
*   **Git-Lore:** Crear instantánea segura: `git-lore checkpoint / git-lore status`
*   **MCP Server:** Validaciones de estado: `git_lore_memory_preflight("commit")`
*   **Git-Lore:** Agentes emiten eventos cortos: `git-lore signal --agent "Codegen"`

> **¿Cómo ayuda?** Permite la colaboración segura (Safe Writes) con Inteligencia Artificial. Con verificaciones previas como `transition_preview` y `preflight`, se evita la sobrescritura y entropía donde la IA accidentalmente contradiga decisiones base de otras ramas.

## 8. Congelando el Conocimiento (Checkpoints)
Estás a punto de hacer un refactor masivo de reglas de negocio o estás orquestando múltiples agentes de IA simultáneos. Necesitas asegurar un punto de restauración seguro de las intenciones de tu equipo.

**Flujo:**
*   **Git-Lore:** Congelar el estado base: `git-lore checkpoint --message "Pre-refactor de auth"`
*   **MCP Server:** Agentes IA validan checksums: `git_lore_state_snapshot()`
*   **Integración:** Fallo rápido (Fail-fast) preventivo en caso de discrepancias temporales.

> **¿Cómo ayuda?** Resuelve la desalineación de estados o condiciones de carrera entre ramas, humanos y Agentes de IA. Un 'checkpoint' crea una fotografía instantánea del *Lore*. Si el código muta o un agente propone un cambio basándose en información desactualizada, el archivo bloquea la sobre-escritura (Strict State-First Check).

## 9. Fusión y Reconciliación de Conocimiento (Merge)
Trabajas en una rama feature donde propusiste nuevas decisiones, mientras que en la rama 'main' otra persona agregó o deprecó otras reglas. Ahora necesitas fusionar ambas ramas sin perder ni contradecir el Lore.

**Flujo:**
*   **Git:** Comienza la fusión de archivos: `git merge feature/branch`
*   **Git-Lore:** Git dispara el merge driver: `git-lore merge <base> <current> <other>`
*   **Git-Lore:** Reconciliación: deduplica IDs, verifica estados (Ej. "Accepted" vence a "Proposed").

> **¿Cómo ayuda?** Git-Lore se instala como un 'Merge Driver' personalizado (vía `git-lore install`). A diferencia de fusionar código o JSON manualmente, este previene colisiones semánticas. Si un átomo en 'main' fue marcado como `Deprecated`, pero en tu rama lo habías actualizado, el algoritmo de reconciliación lo fusionará inteligentemente.

## 10. Proposiciones y Señales Contextuales (Propose & Signal)
Durante un sprint rápido, un desarrollador o una IA lanza una "Suposición" temporal (Signal) al aire para que la IA que trabaje en el código asociado la tenga en cuenta temporalmente, o "proponga" formalmente (Propose) una nueva convención.

**Flujo:**
*   **Git-Lore:** Crear señal efímera: `git-lore signal --assumption "Asumo que la API devuelve XML" --path src/`
*   **MCP Server:** Subagentes leen la señal: `git_lore_memory_search()` expone la suposición fresca.
*   **Git-Lore:** Validación: `git-lore propose --title "API responde JSON" --kind decision` reemplaza la suposición.

> **¿Cómo funciona el salto de Señal a Decisión internamente?**
>
> 1.  **La Señal (Conocimiento Efímero):** `git-lore signal` NO crea un Registro permanente. Crea un archivo temporal (PrismSignal) con un Tiempo de Vida (TTL) programado para expirar. Actúa como un cerrojo suave ("Soft-lock") para avisar a otros agentes: *"Ojo, estoy asumiendo esto en la memoria ahora mismo"*.
> 2.  **La Decisión (Conocimiento Canónico):** `git-lore propose --kind decision` crea un "Átomo" real, un archivo JSON estructurado con un UUID que entra formalmente al ciclo de evaluación (Proposed / Accepted).
> 3.  **El Reemplazo:** La "asunción" inicial no se sobre-escribe mágicamente código sobre código. En cambio, cuando el agente termina su trabajo y formaliza la regla con `propose`, el servidor inscribe el Átomo permanente. En procesos de guardado posteriores, Git-Lore invoca una limpieza (`prune_stale_prism_signals`) evaporando las señales vencidas de la carpeta `.lore/signals/`. El conocimiento fugaz muere, y el canon estructurado prevalece inmutable.
</instructions>
