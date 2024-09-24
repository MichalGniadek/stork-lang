const fs = require("fs");
const toml = require("toml");
const chokidar = require("chokidar");

function tomlToJson(filePath) {
  try {
    const tomlContent = fs.readFileSync(filePath, "utf-8");

    let jsonObj;
    try {
      jsonObj = toml.parse(tomlContent);
    } catch (e) {
      console.error(
        "Parsing error on line " +
          e.line +
          ", column " +
          e.column +
          ": " +
          e.message
      );
      return;
    }

    const jsonString = JSON.stringify(jsonObj, null, 2);

    const outputFilePath = filePath.replace(/\.toml$/, ".json");
    fs.writeFileSync(outputFilePath, jsonString);
  } catch (error) {}
}

// Get file path from command line arguments
const [, , filePath] = process.argv;

if (!filePath) {
  console.error("Please provide a TOML file path as an argument.");
  process.exit(1);
}

tomlToJson(filePath);

chokidar.watch(filePath).on("change", (path) => {
  console.log(`File changed: ${path}`);
  tomlToJson(path);
});
