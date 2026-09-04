/** Display-only unified inbox item projected from kernel-owned source truth. */
export interface ProductivityInboxProjection { readonly nativeId:string;readonly providerId:string;readonly sourceCitation:string;readonly freshness:string;readonly classification:string;readonly coverageGap:string|null;readonly suggestionLabel:"deterministic"|"user_rule"|"model_suggestion"; }
/** Accessible status text announces dynamic truth without provider-write authority. */
export function inboxStatus(item:ProductivityInboxProjection):string{const gap=item.coverageGap===null?"coverage current":`coverage gap: ${item.coverageGap}`;return `${item.providerId}; ${item.freshness}; ${gap}; ${item.suggestionLabel}`;}
