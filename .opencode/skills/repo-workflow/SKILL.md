---
name: repo-workflow
description: Flujo de branching y PR obligatorio para el proyecto nya. Usar SIEMPRE antes de hacer cualquier cambio en el código o docs del repo: verificar estado de git, crear rama feature/, fix/ o docs/ desde main, implementar sin commitear, y hacer push+PR solo bajo petición explícita del usuario. Aplica también al post-merge (volver a main, pull, borrar rama local).
---

# repo-workflow

Flujo de ramas y PRs para `nya`. Regla de hierro: si el usuario no lo pidió
explícitamente → no hay commit, no hay push, no hay PR.

## 1. Verificación previa (antes de tocar cualquier archivo)

1. Ejecutar `git status` y `git branch --show-current`.
2. Si hay cambios sin commitear ajenos a la tarea: reportarlos y no tocarlos.
3. Si se está en `main`, crear una rama según el alcance:
   - `feature/<nombre-corto>` — funcionalidad nueva
   - `fix/<nombre-corto>` — corrección de bug
   - `docs/<nombre-corto>` — solo documentación (README, AGENTS.md, etc.)
   - El prefijo DEBE coincidir con los triggers del CI (`feature/**`,
     `fix/**`, `docs/**`), si no, el push no validará nada.
4. Si ya se está en una rama `feature/`, `fix/` o `docs/` del trabajo actual,
   continuar en ella.
5. Si el estado es ambiguo (main con varias ramas, rama de otro trabajo),
   preguntar al usuario antes de decidir.

## 2. Durante la implementación

- Trabajar solo en la rama creada.
- NO ejecutar `git commit` salvo petición explícita del usuario.
- NO hacer push ni abrir PRs por iniciativa propia.
- Al terminar: resumen de lo hecho + `git status` (archivos modificados sin
  commitear a la espera de decisión del usuario).

## 3. Cuando el usuario acepta explícitamente (pide commit)

1. `git add` solo los archivos intendados (revisar `git status`/`git diff`
   antes; nada de `git add .` a ciegas).
2. Commit con Conventional Commits en inglés:
   `feat(scope): ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `chore: ...`.
3. `git push -u origin <rama>`.
4. Abrir el PR a `main` con `gh pr create`: título Conventional Commit,
   descripción breve del cambio y checks locales ejecutados.

## 4. Cuando el PR se mergea

1. `git checkout main`
2. `git pull`
3. `git branch -d <rama>` (local; `-d` falla si algo quedó sin merge, lo cual
   es una señal de alerta, no un error a saltar con `-D`).
4. No borrar la rama remota con `git push --delete` sin pedirlo; si GitHub la
   conserva, se elimina desde la UI o el usuario lo decide.
