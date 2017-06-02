var fs = require("fs");

var gulp = require("gulp");
var sass = require("gulp-sass");
var postcss = require("gulp-postcss");
var lost = require("lost");
var autoprefixer = require("autoprefixer");

var sassFiles = "./sass/**/*.scss";


gulp.task("sass", function() {
  var processors = [
    lost,
    autoprefixer({ browsers: ["last 2 versions"] })
  ];

  return gulp.src(sassFiles, {base: "./sass"})
    .pipe(sass({outputStyle: "compressed"}).on("error", sass.logError))
    .pipe(postcss(processors))
    .pipe(gulp.dest("./static/"))
});

gulp.task("watch", function() {
  gulp.watch(sassFiles, gulp.parallel("sass"));
});
