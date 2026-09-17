import { z } from "zod";
import { GridSchema, ImageRefSchema, LinkSchema, InlineLocationContentSchema, FogOfWarSchema, type HexenProject } from "@hex-enductor/hexen-schema";
import { saveLink, saveLocationContent, addLocationLink, saveGrid, saveImage, setFog, setFogCells } from "./mutations.ts";

const LinkPatchSchema = LinkSchema.partial();
const InlineContentPatchSchema = InlineLocationContentSchema.omit({ type: true }).partial();

export const SaveLinkCommandSchema = z.object({
  type: z.literal("saveLink"),
  locationId: z.string(),
  linkId: z.string(),
  patch: LinkPatchSchema,
});

export const SaveLocationContentCommandSchema = z.object({
  type: z.literal("saveLocationContent"),
  locationId: z.string(),
  patch: InlineContentPatchSchema,
});

// The pin's own marker type (settlement/ruin/landmark/...) is named
// `linkType` here, not `type` — `type` is already the command envelope's
// own discriminant.
export const AddLocationLinkCommandSchema = z.object({
  type: z.literal("addLocationLink"),
  parentLocationId: z.string(),
  targetLocationId: z.string().min(1),
  x: z.number(),
  y: z.number(),
  linkType: z.string().min(1),
});

export const SaveGridCommandSchema = z.object({
  type: z.literal("saveGrid"),
  locationId: z.string(),
  grid: GridSchema.nullable(),
});

export const SaveImageCommandSchema = z.object({
  type: z.literal("saveImage"),
  locationId: z.string(),
  image: ImageRefSchema.nullable(),
});

export const SetFogCommandSchema = z.object({
  type: z.literal("setFog"),
  locationId: z.string(),
  fog: FogOfWarSchema.nullable(),
});

export const SetFogCellsCommandSchema = z.object({
  type: z.literal("setFogCells"),
  locationId: z.string(),
  cells: z.array(z.string()),
  revealed: z.boolean(),
});

/**
 * Every interaction that mutates a HexenProject, as data — the same
 * shape whether it arrives over hexend's WS session or is applied
 * straight to an in-memory project by the local-fs storage backend.
 * Keeping the union and its reducer here (next to the pure mutation
 * functions) means both transports validate and apply commands
 * identically.
 */
export const CommandSchema = z.discriminatedUnion("type", [
  SaveLinkCommandSchema,
  SaveLocationContentCommandSchema,
  AddLocationLinkCommandSchema,
  SaveGridCommandSchema,
  SaveImageCommandSchema,
  SetFogCommandSchema,
  SetFogCellsCommandSchema,
]);
export type Command = z.infer<typeof CommandSchema>;

export function applyCommand(project: HexenProject, command: Command): HexenProject {
  switch (command.type) {
    case "saveLink":
      return saveLink(project, command.locationId, command.linkId, command.patch);
    case "saveLocationContent":
      return saveLocationContent(project, command.locationId, command.patch);
    case "addLocationLink":
      return addLocationLink(
        project,
        command.parentLocationId,
        command.targetLocationId,
        command.x,
        command.y,
        command.linkType,
      );
    case "saveGrid":
      return saveGrid(project, command.locationId, command.grid);
    case "saveImage":
      return saveImage(project, command.locationId, command.image);
    case "setFog":
      return setFog(project, command.locationId, command.fog);
    case "setFogCells":
      return setFogCells(project, command.locationId, command.cells, command.revealed);
  }
}
