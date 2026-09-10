// Recent projects opened via the File System Access API — a
// directory handle isn't a string, so it can't live in localStorage
// next to the server path list (see recentProjects.ts). IndexedDB can
// store a handle directly (Chromium supports structured-cloning it),
// which is the whole point of keeping this separate.

const DB_NAME = "hex-enductor-local-recents";
const STORE = "handles";
const MAX_ENTRIES = 5;

export interface LocalRecentEntry {
  name: string;
  handle: FileSystemDirectoryHandle;
  openedAt: number;
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1);
    req.onupgradeneeded = () => {
      req.result.createObjectStore(STORE, { keyPath: "name" });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

function getAll(db: IDBDatabase): Promise<LocalRecentEntry[]> {
  return new Promise((resolve, reject) => {
    const req = db.transaction(STORE, "readonly").objectStore(STORE).getAll();
    req.onsuccess = () => resolve((req.result as LocalRecentEntry[]).sort((a, b) => b.openedAt - a.openedAt));
    req.onerror = () => reject(req.error);
  });
}

export async function addLocalRecent(handle: FileSystemDirectoryHandle): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).put({ name: handle.name, handle, openedAt: Date.now() });
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });

  const all = await getAll(db);
  const excess = all.slice(MAX_ENTRIES);
  if (excess.length > 0) {
    const tx = db.transaction(STORE, "readwrite");
    for (const entry of excess) tx.objectStore(STORE).delete(entry.name);
  }
}

export async function getLocalRecents(): Promise<LocalRecentEntry[]> {
  try {
    return await getAll(await openDb());
  } catch {
    return [];
  }
}
