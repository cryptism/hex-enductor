import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { z } from "zod";
import { LinkSchema, InlineLocationContentSchema, GridSchema, ImageRefSchema } from "@hex-enductor/hexen-schema";
import {
  createMinimalProject,
  saveLink as applySaveLink,
  saveLocationContent as applySaveLocationContent,
  addLocationLink as applyAddLocationLink,
  saveGrid as applySaveGrid,
  saveImage as applySaveImage,
} from "@hex-enductor/project-ops";
import { router, publicProcedure } from "./trpc.ts";
import { openProject, saveProject } from "./projectIO.ts";
import { listDirectory } from "./browse.ts";

const LinkPatchSchema = LinkSchema.partial();
const InlineContentPatchSchema = InlineLocationContentSchema.omit({ type: true }).partial();

export const appRouter = router({
  openProject: publicProcedure.input(z.object({ path: z.string() })).query(({ input }) => {
    return openProject(input.path);
  }),

  createProject: publicProcedure
    .input(
      z.object({
        path: z.string(),
        title: z.string().min(1),
        defaultLocationId: z.string().min(1),
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

      // No vault to choose or scaffold — inline content means a new
      // project is immediately valid with nothing but a name.
      const project = createMinimalProject(input.title, input.defaultLocationId);

      await saveProject(input.path, project);
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
      applySaveLink(project, input.locationId, input.linkId, input.patch);
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
      applySaveLocationContent(project, input.locationId, input.patch);
      await saveProject(input.path, project);
      return openProject(input.path);
    }),

  addLocationLink: publicProcedure
    .input(
      z.object({
        path: z.string(),
        parentLocationId: z.string(),
        targetLocationId: z.string().min(1),
        x: z.number(),
        y: z.number(),
        type: z.string().min(1),
      }),
    )
    .mutation(async ({ input }) => {
      const { project } = await openProject(input.path);
      applyAddLocationLink(project, input.parentLocationId, input.targetLocationId, input.x, input.y, input.type);
      await saveProject(input.path, project);
      return openProject(input.path);
    }),

  saveGrid: publicProcedure
    .input(
      z.object({
        path: z.string(),
        locationId: z.string(),
        grid: GridSchema.nullable(),
      }),
    )
    .mutation(async ({ input }) => {
      const { project } = await openProject(input.path);
      applySaveGrid(project, input.locationId, input.grid);
      await saveProject(input.path, project);
      return openProject(input.path);
    }),

  saveImage: publicProcedure
    .input(
      z.object({
        path: z.string(),
        locationId: z.string(),
        image: ImageRefSchema.nullable(),
      }),
    )
    .mutation(async ({ input }) => {
      const { project } = await openProject(input.path);
      applySaveImage(project, input.locationId, input.image);
      await saveProject(input.path, project);
      return openProject(input.path);
    }),
});

export type AppRouter = typeof appRouter;
