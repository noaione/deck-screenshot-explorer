import { fileURLToPath, URL } from "node:url";

import { defineConfig, splitVendorChunk } from "vite";
import vue from "@vitejs/plugin-vue";
import vueDevTools from "vite-plugin-vue-devtools";
import Components from "unplugin-vue-components/vite";
import VueRouter from "unplugin-vue-router/vite";
import { VueUseComponentsResolver, VueUseDirectiveResolver } from "unplugin-vue-components/resolvers";
import Icons from "unplugin-icons/vite";
import IconsResolver from "unplugin-icons/resolver";
import { VueRouterAutoImports } from "unplugin-vue-router";
import { unheadVueComposablesImports } from "@unhead/vue";
import AutoImport from "unplugin-auto-import/vite";
import type { ManualChunkMeta } from "rollup";
import browserslist from "browserslist";
import { browserslistToTargets } from "lightningcss";

function splitMoreVendorChunk(
  id: string,
  getModuleInfo: ManualChunkMeta,
  extraSplitName?: (id: string) => string | undefined
): string | undefined {
  const isVendorChunk = splitVendorChunk();

  if (isVendorChunk(id, getModuleInfo)) {
    if (typeof extraSplitName === "function") {
      const extraName = extraSplitName(id);

      if (extraName) {
        return `vendor.${extraName}`;
      }
    }

    return "vendor";
  }
}

// https://vitejs.dev/config/
export default defineConfig({
  appType: "spa",
  plugins: [
    VueRouter({
      routesFolder: "src/pages",
      extensions: [".vue"],
      dts: "./src/types/router.d.ts",
    }),
    vue({
      script: {
        defineModel: true,
        propsDestructure: true,
      },
    }),
    vueDevTools(),
    Icons({
      compiler: "vue3",
      defaultClass: "v-icon",
    }),
    Components({
      dts: "./src/types/components.d.ts",
      resolvers: [IconsResolver(), VueUseComponentsResolver(), VueUseDirectiveResolver()],
    }),
    AutoImport({
      imports: ["vue", VueRouterAutoImports, unheadVueComposablesImports],
      dirs: ["./src/composables/**"],
      dts: "./src/types/imports.d.ts",
    }),
  ],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("src", import.meta.url)),
    },
  },
  build: {
    cssMinify: "lightningcss",
    cssCodeSplit: false,
    target: "es2022",
    modulePreload: false,
    rollupOptions: {
      output: {
        manualChunks: (id, getModuleInfo) => {
          if (id.includes("src/assets/")) {
            // get everything after "src/assets/"
            const [, assetPath] = id.split("src/assets/");
            // remove the extension
            const [assetName] = assetPath.split(".");

            return `meta/${assetName.replace("/", ".").replace("_", ".")}`;
          }

          if (id.includes("/pages") && !id.startsWith("virtual:")) {
            const [, routesPath] = id.split("src/pages/");
            const [routesName] = routesPath.split(".");
            const safeRoutesName = routesName.replace("_", ".").replace("[", "_").replace("]", "");

            return `pages/${safeRoutesName}`;
          }

          return splitMoreVendorChunk(id, getModuleInfo, (intId) => {
            const isVueRelated = ["@vue", "vue-i18n", "@intlify", "vue-router"].some((name) => {
              return intId.includes(name);
            });

            if (isVueRelated) {
              return "vue";
            }

            if (intId.includes("lodash")) {
              return "lodash";
            }
          });
        },
      },
    },
    sourcemap: true,
  },
  css: {
    lightningcss: {
      nonStandard: {
        deepSelectorCombinator: true,
      },
      targets: browserslistToTargets(browserslist()),
    },
  },
});
