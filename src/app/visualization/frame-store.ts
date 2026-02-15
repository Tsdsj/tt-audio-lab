import { shallowRef } from "vue";

export const spectrumFrameBins = shallowRef<number[]>(new Array(64).fill(0));
