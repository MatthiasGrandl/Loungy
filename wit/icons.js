#!/usr/bin/env bun

import { Glob } from "bun";
const glob = new Glob("./assets/icons/*.svg");

// Scans the current working directory and each of its sub-directories recursively
let icons = [];
for await (const path of glob.scan(".")) {
  if (path.endsWith(".svg")) {
    const name = path.split("/").pop().replace(".svg", "");
    icons.push(
      `%${name
        .split("-")
        .map((seg) => {
          // Prefix any segment that starts with a number with a 'wit'
          return seg.match(/^\d/) ? `wit${seg}` : seg;
        })
        .join("-")}`
    );
  }
}

const output = `package loungy:command;

interface icons {
  enum icon {
    ${icons.map((icon) => `${icon},`).join("\n    ")}
  }
}
`;

Bun.write("./wit/icons.wit", output);
