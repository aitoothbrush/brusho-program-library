// This is a custom fetch function that uses axios to make requests and retries on errors or 502 status codes
// Run this file by executing `npx tsx axiosFetchWithRetries.ts`

import { Connection } from "@solana/web3.js";
import axios from "axios";
import * as https from 'https'

const RETRY_ATTEMPTS = 3;

const agent = new https.Agent({
  maxSockets: 100,
});

const axiosObject = axios.create({
  httpsAgent: agent,
});

export async function axiosFetchWithRetries(
  input: string | URL | globalThis.Request,
  init?: RequestInit,
  retryAttempts = 0
): Promise<Response> {
  let attempt = 0;

  // Adding default headers
  if (!init || !init.headers) {
    init = {
      headers: {
        "Content-Type": "application/json",
      },
      ...init,
    };
  }

  while (attempt < retryAttempts) {
    try {
      let axiosHeaders = {};

      axiosHeaders = Array.from(new Headers(init.headers).entries()).reduce(
        (acc, [key, value]) => {
          (acc as any)[key] = value;
          return acc;
        },
        {}
      );

      const axiosConfig = {
        data: init.body,
        headers: axiosHeaders,
        method: init.method,
        baseURL: input.toString(),
        validateStatus: (_status: any) => true,
      };

      const axiosResponse = await axiosObject.request(axiosConfig);

      const { data, status, statusText, headers } = axiosResponse;

      // Mapping headers from axios to fetch format
      const headersArray: [string, string][] = Object.entries(headers).map(
        ([key, value]) => [key, value]
      );

      const fetchHeaders = new Headers(headersArray);

      const response = new Response(JSON.stringify(data), {
        status,
        statusText,
        headers: fetchHeaders,
      });

      // Comment the above lines and uncomment the following one to switch from axios to fetch
      // const response = await fetch(input, init);

      // Traffic might get routed to backups or node restarts or if anything throws a 502, retry
      if (response.status === 502) {
        console.log("Retrying due to 502");

        attempt++;

        // Backoff to avoid hammering the server
        await new Promise<void>((resolve) =>
          setTimeout(resolve, 100 * attempt)
        );

        continue;
      }
      return Promise.resolve(response);
    } catch (e) {
      console.log(`Retrying due to error ${e}`, e);

      attempt++;
      continue;
    }
  }

  return Promise.reject("Max retries reached");
}

export function getConnection(endpoint: string) {
  return new Connection(endpoint, {
    async fetch(input, init?) {
      console.log(
        "Custom fetch function",
        input,
        (init as any).method,
        (init as any).body,
        (init as any).headers
      );

      return await axiosFetchWithRetries(input, init, RETRY_ATTEMPTS);
    },
  });
}
