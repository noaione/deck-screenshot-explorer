import { defineStore } from "pinia";

export interface DeckMeta {
  /**
   * The per_page settings for the pagination
   */
  per_page: 10 | 20 | 50 | 100;
}

export const useDeckStore = defineStore("decky", {
  state: (): DeckMeta => ({
    per_page: 10,
  }),
  actions: {
    setPerPage(per_page: 10 | 20 | 50 | 100): void {
      this.per_page = per_page;
    },
    reset(): void {
      this.per_page = 10;
    },
  },
});
