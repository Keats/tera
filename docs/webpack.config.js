const path = require('path');

module.exports = {
  mode: "development",
  entry: "./bootstrap.js",
  output: {
    path: path.resolve(__dirname, "static", "playground"),
    filename: "bootstrap.js",
  },
};
