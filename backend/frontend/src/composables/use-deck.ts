import { defineStore } from "pinia";

export interface DeckMeta {
  /**
   * The user ID3 of the deck.
   */
  user?: number;
  /**
   * The currently selected game/app
   */
  app?: number;
  /**
   * The per_page settings for the pagination
   */
  per_page?: 10 | 20 | 50 | 100;
}

export const useDeckStore = defineStore("decky", {
  state: (): DeckMeta => ({
    per_page: 10,
  }),
  actions: {
    /**
     * Set the user ID3 of the deck.
     * @param user The user ID3 of the deck.
     */
    setUser(user: number): void {
      this.user = user;
    },
    /**
     * Set the currently selected game/app.
     * @param app The currently selected game/app.
     */
    setApp(app: number): void {
      this.app = app;
    },
    /**
     * Set the per_page settings for the pagination.
     * @param per_page The per_page settings for the pagination.
     */
    setPerPage(per_page: 10 | 20 | 50 | 100): void {
      this.per_page = per_page;
    },
    reset(): void {
      this.user = undefined;
      this.app = undefined;
      this.per_page = 10;
    },
  },
});
