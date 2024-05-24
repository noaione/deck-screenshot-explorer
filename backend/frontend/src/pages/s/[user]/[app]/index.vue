<template>
  <HeaderView />
  <main class="flex w-full flex-col px-2 py-4">
    <div class="flex px-2">
      <RouterLink :to="`/s/${$route.params.user}`" class="flex flex-row gap-2 transition-opacity hover:opacity-80">
        <i-mdi-arrow-left class="h-6 w-6" />
        Go Back
      </RouterLink>
    </div>

    <LoadingView v-if="loading" />
    <div v-else class="mt-4 flex w-full flex-col gap-2 md:flex-row md:flex-wrap">
      <DeckApp v-if="appInfo" :app="appInfo" />

      <div class="flex px-4 text-lg font-semibold">Screenshots</div>
      <div
        v-if="availableScreenshot.length === 0"
        class="my-4 flex w-full flex-col justify-center text-center text-lg opacity-80"
      >
        No Screenshot
      </div>
      <div v-else class="flex w-full flex-col gap-2 md:flex-row md:flex-wrap md:justify-center">
        <DeckSS
          v-for="screenshot in availableScreenshot"
          :key="screenshot"
          :userId="$route.params.user"
          :appId="$route.params.app"
          :filename="screenshot"
        />
      </div>
      <DeckPaging v-if="pagination" :page="currentPage" :total="pagination.total" @update="updatePage" />
    </div>
  </main>
</template>

<script setup lang="ts">
import type { AppInfo, AppInfoWithScreenshots, Pagination } from "@/composables/use-backend-fetch";
import { watch } from "vue";

const deck = useDeckStore();
const appInfo = ref<AppInfo>();
const availableScreenshot = ref<string[]>([]);
const currentPage = ref(0);
const pagination = ref<Pagination>();
const loading = ref(true);
const route = useRoute();

async function doRequest(userId: string, appId: string) {
  loading.value = true;

  const request = await useBackendFetch<AppInfoWithScreenshots>(
    `/users/${userId}/${appId}?page=${currentPage.value}&per_page=${deck.per_page}`
  );

  availableScreenshot.value = request.screenshots;
  appInfo.value = request.app;
  pagination.value = request.pagination;
  loading.value = false;

  return request;
}

async function updatePage(page: number) {
  currentPage.value = page;

  // @ts-ignore
  await doRequest(route.params.user, route.params.app);
}

watch(
  // @ts-ignore
  () => route.params.app,
  async (newId, _) => {
    // @ts-ignore
    const userId = route.params.user;

    const request = await doRequest(userId, newId);

    useHeadSafe({
      // @ts-ignore
      title: `${request.app.name} :: Deck Screenshot Explorer`,
    });
  },
  {
    immediate: true,
  }
);
</script>
