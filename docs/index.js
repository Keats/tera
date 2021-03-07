import * as wasm from "tera-web";


function renderTera(template, context) {
  try {
    const value = wasm.render(template, context);
    return {value, error: null};
  } catch (error) {
    return {value: null, error};
  }
}
window.renderTera = renderTera;
