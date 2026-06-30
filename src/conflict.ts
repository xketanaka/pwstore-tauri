export type ConflictChoice = "local" | "drive" | "cancel";

export function showConflictDialog(): Promise<ConflictChoice> {
  return new Promise((resolve) => {
    const dialog = document.querySelector<HTMLElement>("#conflict-dialog")!;
    dialog.hidden = false;

    const pick = (choice: ConflictChoice) => {
      dialog.hidden = true;
      resolve(choice);
    };

    document.querySelector<HTMLButtonElement>("#conflict-use-local")!
      .addEventListener("click", () => pick("local"), { once: true });
    document.querySelector<HTMLButtonElement>("#conflict-use-drive")!
      .addEventListener("click", () => pick("drive"), { once: true });
    document.querySelector<HTMLButtonElement>("#conflict-cancel")!
      .addEventListener("click", () => pick("cancel"), { once: true });
  });
}
