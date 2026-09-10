import { z } from "zod";
import { LinkSchema, InlineLocationContentSchema } from "@hex-enductor/hexen-schema";
import { router, publicProcedure } from "./trpc.ts";
import { openProject, saveProject } from "./projectIO.ts";
import { listDirectory } from "./browse.ts";

const LinkPatchSchema = LinkSchema.partial();
const InlineContentPatchSchema = InlineLocationContentSchema.omit({ type: true }).partial();

export const appRouter = router({
  openProject: publicProcedure.input(z.object({ path: z.string() })).query(({ input }) => {
    return openProject(input.path);
  }),

  listDirectory: publicProcedure.input(z.object({ path: z.string().optional() })).query(({ input }) => {
    return listDirectory(input.path);
  }),

  saveLink: publicProcedure
    .input(
      z.object({
        path: z.string(),
        locationId: z.string(),
        linkId: z.string(),
        patch: LinkPatchSchema,
      }),
    )
    .mutation(async ({ input }) => {
      const { project } = await openProject(input.path);

      const location = project.locations.find((l) => l.id === input.locationId);
      if (!location) {
        throw new Error(`No location "${input.locationId}" in ${input.path}`);
      }
      const linkIndex = location.links.findIndex((l) => l.id === input.linkId);
      if (linkIndex === -1) {
        throw new Error(`Location "${input.locationId}" has no link "${input.linkId}"`);
      }

      location.links[linkIndex] = { ...location.links[linkIndex]!, ...input.patch };

      await saveProject(input.path, project);
      return openProject(input.path);
    }),

  saveLocationContent: publicProcedure
    .input(
      z.object({
        path: z.string(),
        locationId: z.string(),
        patch: InlineContentPatchSchema,
      }),
    )
    .mutation(async ({ input }) => {
      const { project } = await openProject(input.path);

      const location = project.locations.find((l) => l.id === input.locationId);
      if (!location) {
        throw new Error(`No location "${input.locationId}" in ${input.path}`);
      }
      if (location.content !== null && location.content.type !== "inline") {
        throw new Error(
          `Location "${input.locationId}" has ${location.content.type} content, not inline`,
        );
      }

      location.content = {
        type: "inline",
        title: location.content?.title ?? "",
        body: location.content?.body ?? "",
        ...input.patch,
      };

      await saveProject(input.path, project);
      return openProject(input.path);
    }),
});

export type AppRouter = typeof appRouter;
