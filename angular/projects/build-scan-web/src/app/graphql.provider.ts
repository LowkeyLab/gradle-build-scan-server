import { inject } from '@angular/core';
import { provideApollo } from 'apollo-angular';
import { HttpLink } from 'apollo-angular/http';
import { InMemoryCache } from '@apollo/client/core';

const GRAPHQL_URI = new URL('graphql', document.baseURI).toString();

export function provideGraphql() {
  return provideApollo(() => {
    const httpLink = inject(HttpLink);
    return {
      link: httpLink.create({ uri: GRAPHQL_URI }),
      cache: new InMemoryCache({
        typePolicies: {
          Query: {
            fields: {
              buildScans: {
                keyArgs: false,
                merge(existing: any, incoming: any) {
                  if (!existing) return incoming;
                  return {
                    ...incoming,
                    edges: [...existing.edges, ...incoming.edges],
                  };
                },
              },
            },
          },
        },
      }),
    };
  });
}
