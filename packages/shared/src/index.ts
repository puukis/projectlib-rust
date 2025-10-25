import { z } from "zod";

export const PingSchema = z.object({
  message: z.string()
});

export type Ping = z.infer<typeof PingSchema>;

export * from "./git";
