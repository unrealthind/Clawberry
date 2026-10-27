import { request, type Agent } from "./api";

// This function intentionally has no query/mutation cache, storage, or logging.
export function authorizeWrite(
  token: string,
  owner: Agent,
  reviewId: string,
  reviewedHash: string,
  code: string,
) {
  if (!/^[0-9]{6}$/.test(code)) throw new Error("Invalid authorization code");
  return request<{ session_id: string; authorized: boolean }>(
    token,
    `/api/agents/${owner}/write-authorizations`,
    { review_id: reviewId, reviewed_hash: reviewedHash, code },
  );
}
