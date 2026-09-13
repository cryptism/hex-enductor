import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { z } from "zod";
import { ProjectContentSchema } from "@hex-enductor/hexen-schema";
import { createMinimalProject } from "@hex-enductor/project-ops";
import { router, publicProcedure } from "./trpc.ts";
import { openProject, saveProject } from "./projectIO.ts";
import { listDirectory } from "./browse.ts";

// Everything that touches a project's live state (opening it for
// editing, and every mutation) goes through the /ws session protocol
// in server.ts instead — hexend is authoritative there, and that's the
// only path that can push a change out to more than one connected
// client. What's left here is one-shot filesystem operations that
// don't have "current state" to be authoritative over.
export const appRouter = router({
  createProject: publicProcedure
    .input(
      z.object({
        path: z.string(),
        title: z.string().min(1),
        defaultLocationId: z.string().min(1),
        content: ProjectContentSchema.default({ type: "inline" }),
      }),
    )
    .mutation(async ({ input }) => {
      if (existsSync(input.path)) {
        throw new Error(`"${input.path}" already exists`);
      }
      const dir = dirname(input.path);
      if (!existsSync(dir)) {
        throw new Error(`Directory "${dir}" doesn't exist`);
      }

      const project = createMinimalProject(input.title, input.defaultLocationId, input.content);

      await saveProject(input.path, project);
      return openProject(input.path);
    }),

  listDirectory: publicProcedure.input(z.object({ path: z.string().optional() })).query(({ input }) => {
    return listDirectory(input.path);
  }),
});

export type AppRouter = typeof appRouter;
