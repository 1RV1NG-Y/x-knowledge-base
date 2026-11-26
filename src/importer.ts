import { importArchive, pickArchive } from './api';
import { desk, message } from './state.svelte';

/** Pick an archive (the ZIP from X or its extracted folder) and import it. */
export async function importFrom(directory: boolean) {
  if (desk.importing) return;
  desk.importError = undefined;
  const path = await pickArchive(directory);
  if (!path) return;
  desk.importing = true;
  desk.importSummary = undefined;
  try {
    const running = importArchive(path);
    void desk.refreshJob();
    desk.importSummary = await running;
    await Promise.all([desk.refreshProfiles(), desk.refreshPlaylists(), desk.refreshJob()]);
    desk.libraryVersion += 1;
    const s = desk.importSummary;
    desk.showToast(`Imported: ${s.imported.toLocaleString()} new, ${s.existing.toLocaleString()} already here`);
  } catch (cause) {
    desk.importError = message(cause);
  } finally {
    desk.importing = false;
  }
}
