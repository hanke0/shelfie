import { defineConfig } from "orval";

export default defineConfig({
  shelfie: {
    input: "../openapi/openapi.json",
    output: {
      mode: "tags-split",
      target: "src/api/generated",
      schemas: "src/api/generated/models",
      client: "react-query",
      httpClient: "axios",
      clean: true,
      override: {
        mutator: {
          path: "src/lib/custom-fetch.ts",
          name: "customFetch",
        },
      },
    },
  },
});
