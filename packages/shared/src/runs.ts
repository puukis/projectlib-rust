import { z } from "zod";

export const RunDetectionSchema = z.object({
  language: z.string(),
  command: z.string(),
  args: z.array(z.string()),
  env: z.record(z.string(), z.string()),
  cwd: z.string().nullable().optional(),
  reason: z.string()
});

export type RunDetection = z.infer<typeof RunDetectionSchema>;

export const RunDetectionListSchema = z.array(RunDetectionSchema);
