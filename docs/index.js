import * as wasm from "tera-web";

let template = document.getElementById("template");
let context = document.getElementById("context");
let rendered = document.getElementById("rendered");
let errors = document.getElementById("errors");

function render() {
  try {
    rendered.value = wasm.render(template.value, context.value);
    errors.textContent = "";
  } catch (error) {
    rendered.value = "";
    errors.textContent = error;
  }
}

function debounce(func, timeout = 300) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => { func.apply(this, args); }, timeout);
  };
}

template.onkeyup = debounce(() => render());
context.onkeyup = debounce(() => render());

render();
