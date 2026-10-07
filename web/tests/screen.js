// Locators for the screen's parts, by their data-* hooks.

export const card = (scope, id) => scope.locator(`[data-card="${id}"]`);
export const row = (scope, id) => scope.locator(`[data-provisional-row="${id}"]`);
export const action = (scope, name) => scope.locator(`[data-action="${name}"]`);
export const mark = (scope, name) => scope.locator(`[data-mark="${name}"]`);
