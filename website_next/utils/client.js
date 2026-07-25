import { BrkClient } from "../modules/brk-client/index.js";

export const BRK_BASE_URL = "http://localhost:3110";
export const brk = new BrkClient(BRK_BASE_URL);
