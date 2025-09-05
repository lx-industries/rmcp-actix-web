## [0.6.1](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.6.0...v0.6.1) (2025-09-05)


### Miscellaneous Chores

* **deps:** update rmcp to version 0.6.3 and fix compilation errors ([73413dc](https://gitlab.com/lx-industries/rmcp-actix-web/commit/73413dc6983312d59b350f49ce87deb70eb423cd)), closes [#28](https://gitlab.com/lx-industries/rmcp-actix-web/issues/28)
* **deps:** update rust crate insta to v1.43.2 ([ccc6cea](https://gitlab.com/lx-industries/rmcp-actix-web/commit/ccc6cea3928029b38b55db6aa2a2f7e201a2443b))

## [0.6.0](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.5.0...v0.6.0) (2025-09-04)


### Features

* add authorization-token-passthrough feature flag ([8ac7ff1](https://gitlab.com/lx-industries/rmcp-actix-web/commit/8ac7ff1aef6ceef2e42bc451971031c985b9a4cc))


### Bug Fixes

* forward Authorization header for existing sessions in stateful mode ([a21d49e](https://gitlab.com/lx-industries/rmcp-actix-web/commit/a21d49edb8720c313b0c150804fb0a0162f392dc))

## [0.5.0](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.4.3...v0.5.0) (2025-09-03)


### Features

* forward Authorization header for MCP proxy scenarios ([36c6cc7](https://gitlab.com/lx-industries/rmcp-actix-web/commit/36c6cc7e0a9b8ba2805f75d6bc1b8d140c7ed7d8))


### Miscellaneous Chores

* **deps:** update rust crate bon to v3.7.2 ([8367e0a](https://gitlab.com/lx-industries/rmcp-actix-web/commit/8367e0a2347d415010a09ec8c60034a5323ce62c))

## [0.4.3](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.4.2...v0.4.3) (2025-09-01)


### Miscellaneous Chores

* **deps:** update node.js to v22.19.0 ([226ed11](https://gitlab.com/lx-industries/rmcp-actix-web/commit/226ed11a3d281b1a31c0afa62cbfbd96abbb8a45))
* **deps:** update rust crate actix-rt to v2.11.0 ([751d11e](https://gitlab.com/lx-industries/rmcp-actix-web/commit/751d11e8607ae9cfcca58fb31999112488cb3921))
* **deps:** update rust crate tracing-subscriber to v0.3.20 ([dd55343](https://gitlab.com/lx-industries/rmcp-actix-web/commit/dd5534353904a4cd31051656de55d0cd3e7df211))

## [0.4.2](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.4.1...v0.4.2) (2025-08-30)


### Miscellaneous Chores

* **deps:** update rust crate rmcp to v0.6.1 ([45bccc2](https://gitlab.com/lx-industries/rmcp-actix-web/commit/45bccc256353da0eddf869013415d70f938e21f1)), closes [#22](https://gitlab.com/lx-industries/rmcp-actix-web/issues/22)
* **deps:** update rust:1.89.0 docker digest to 3329e2d ([551de5b](https://gitlab.com/lx-industries/rmcp-actix-web/commit/551de5b69b3f5dfa2c34610ab60c58913e12160c))

## [0.4.1](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.4.0...v0.4.1) (2025-08-29)


### Bug Fixes

* include scope path in SSE endpoint event URL ([0922c3b](https://gitlab.com/lx-industries/rmcp-actix-web/commit/0922c3ba340693e39266c287a8af29071d24af1b)), closes [#21](https://gitlab.com/lx-industries/rmcp-actix-web/issues/21)

## [0.4.0](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.3.0...v0.4.0) (2025-08-28)


### Features

* update StreamableHttpService API and enable Clone pattern ([f85bcfc](https://gitlab.com/lx-industries/rmcp-actix-web/commit/f85bcfcf38b12ad3abfb7357688d0c9bd59da6ea))

## [0.3.0](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.9...v0.3.0) (2025-08-28)


### Features

* add instance method scope() to StreamableHttpService ([5c5eeed](https://gitlab.com/lx-industries/rmcp-actix-web/commit/5c5eeed9903a75b465f2315e1aa76ea4fa31ae77)), closes [#19](https://gitlab.com/lx-industries/rmcp-actix-web/issues/19)
* implement unified builder pattern for transport services ([fbd0121](https://gitlab.com/lx-industries/rmcp-actix-web/commit/fbd0121af8aa8c867d754ea6ace959b93831a61e))


### Miscellaneous Chores

* **deps:** update rust:1.89.0 docker digest to 26318ae ([b4def23](https://gitlab.com/lx-industries/rmcp-actix-web/commit/b4def23125a3c7db308c1d7258970ce59a072964))

## [0.2.9](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.8...v0.2.9) (2025-08-21)


### Bug Fixes

* calculator example/test return type ([1bea804](https://gitlab.com/lx-industries/rmcp-actix-web/commit/1bea8043edad2dc4cae0d173a6be58e014581dfa))
* wrong HTTP header name for the MCP session id in the examples ([e01257a](https://gitlab.com/lx-industries/rmcp-actix-web/commit/e01257a111a6012c2f42cef359364b0281b462f6))

## [0.2.8](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.7...v0.2.8) (2025-08-21)


### Bug Fixes

* **ci:** run all tests including integration tests ([cd07a8b](https://gitlab.com/lx-industries/rmcp-actix-web/commit/cd07a8b0262ca01b6953471e057f00e9465410a8)), closes [#16](https://gitlab.com/lx-industries/rmcp-actix-web/issues/16)
* session handling inconsistencies in the examples ([9e737dc](https://gitlab.com/lx-industries/rmcp-actix-web/commit/9e737dc2c6d4c01ca939edfca2c6696fb561d692)), closes [#15](https://gitlab.com/lx-industries/rmcp-actix-web/issues/15)


### Miscellaneous Chores

* **deps:** update rust crate rmcp to 0.6.0 ([31b7262](https://gitlab.com/lx-industries/rmcp-actix-web/commit/31b7262bec4ab9327b2a69fc174c0da7ff461509))
* **deps:** update rust crate serde_json to v1.0.143 ([163a043](https://gitlab.com/lx-industries/rmcp-actix-web/commit/163a04381132c48b992d903a1e574b341e8f6bc9))
* **deps:** update rust:1.89.0 docker digest to 6e6d04b ([892b8b5](https://gitlab.com/lx-industries/rmcp-actix-web/commit/892b8b508112767f39de9f3319c5fd94fdcdef2b))

## [0.2.7](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.6...v0.2.7) (2025-08-18)


### Miscellaneous Chores

* **deps:** update node.js to 3266bc9 ([79bd416](https://gitlab.com/lx-industries/rmcp-actix-web/commit/79bd416b141258425a1b869e8ff1ea8d7937a15b))
* **deps:** update node.js to 5cc5271 ([b16e53f](https://gitlab.com/lx-industries/rmcp-actix-web/commit/b16e53f4781122ce3033c77ffa6a7c475e2a53f5))
* **deps:** update rust crate anyhow to v1.0.99 ([61979a0](https://gitlab.com/lx-industries/rmcp-actix-web/commit/61979a0343fdd3a35a9093eed8d1241740f3b638))
* **deps:** update rust crate reqwest to v0.12.23 ([dbb4e82](https://gitlab.com/lx-industries/rmcp-actix-web/commit/dbb4e829e3c994dc36ea74861829e0829c1f5343))
* **deps:** update rust:1.89.0 docker digest to 5fa1490 ([7d3ec02](https://gitlab.com/lx-industries/rmcp-actix-web/commit/7d3ec022fccd77eb6f7e24459a0dbe86769b3375))
* **deps:** update rust:1.89.0 docker digest to ded0544 ([b2207d3](https://gitlab.com/lx-industries/rmcp-actix-web/commit/b2207d34980ac4149b8888ceba73fbc6c0318e37))
* **deps:** update rust:1.89.0 docker digest to e090f7b ([4e9a3c9](https://gitlab.com/lx-industries/rmcp-actix-web/commit/4e9a3c95a44960d153d5e1b2e6e22dd32996afed))

## [0.2.6](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.5...v0.2.6) (2025-08-11)


### Miscellaneous Chores

* **deps:** update node.js to v22.18.0 ([2aee55e](https://gitlab.com/lx-industries/rmcp-actix-web/commit/2aee55e4c8229c2a2cc013476b78a23aba4dfcf4))
* **deps:** update rust crate rmcp to 0.4.0 ([e084de8](https://gitlab.com/lx-industries/rmcp-actix-web/commit/e084de8fee8b9ceed4adcc8537ba28f69911b3a3))
* **deps:** update rust crate rmcp to 0.5.0 ([7f48c7f](https://gitlab.com/lx-industries/rmcp-actix-web/commit/7f48c7fc47669ba68911861a2c3bd56475ce0205))
* **deps:** update rust crate rmcp to v0.4.1 ([4ff8ad1](https://gitlab.com/lx-industries/rmcp-actix-web/commit/4ff8ad1de2fadf378d1379c53da5626094963016))
* **deps:** update rust docker tag to v1.89.0 ([83434cd](https://gitlab.com/lx-industries/rmcp-actix-web/commit/83434cd12cac89d2c35efc7faca4a9e6d22a350b))

## [0.2.5](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.4...v0.2.5) (2025-08-04)


### Miscellaneous Chores

* **deps:** update rust crate rmcp to v0.3.1 ([7838b24](https://gitlab.com/lx-industries/rmcp-actix-web/commit/7838b24e0011515f093c5f3d9a8cf9bc0f804f09))
* **deps:** update rust crate rmcp to v0.3.2 ([175e47f](https://gitlab.com/lx-industries/rmcp-actix-web/commit/175e47f683afb959c2cc647d7849ad78297f72ca))
* **deps:** update rust crate serde_json to v1.0.142 ([3ffe715](https://gitlab.com/lx-industries/rmcp-actix-web/commit/3ffe7155ac2ea7c5ddd3e196a917442ecb672a88))
* **deps:** update rust crate tokio to v1.47.1 ([6e30b25](https://gitlab.com/lx-industries/rmcp-actix-web/commit/6e30b257050f404375990b8cef859a8c003be18d))
* **deps:** update rust crate tokio-util to v0.7.16 ([7adddfa](https://gitlab.com/lx-industries/rmcp-actix-web/commit/7adddfad9e86556289c4021c84bd6d24923d824c))

## [0.2.4](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.3...v0.2.4) (2025-07-28)


### Bug Fixes

* add .gitlab-ci.yml to rust-changes pattern ([c890b6f](https://gitlab.com/lx-industries/rmcp-actix-web/commit/c890b6fea33e2496352b9c66782898a0bad420ef))


### Miscellaneous Chores

* **deps:** update node.js to 079b6a6 ([e9c8dc6](https://gitlab.com/lx-industries/rmcp-actix-web/commit/e9c8dc6ec0ec5b1de18621616c7d7a93a3004a8d))
* **deps:** update node.js to 37ff334 ([6ba8d7b](https://gitlab.com/lx-industries/rmcp-actix-web/commit/6ba8d7bb70ea15b2feb4c3a5656328c62d847ca5))
* **deps:** update node.js to e515259 ([606eb33](https://gitlab.com/lx-industries/rmcp-actix-web/commit/606eb333b73ba3f73a3b4f17ab0b8fa612d318d1))
* **deps:** update rust crate tokio to v1.47.0 ([d9dea1c](https://gitlab.com/lx-industries/rmcp-actix-web/commit/d9dea1cf77e76df4814d6b30000934ca06a4a159))
* **deps:** update rust:1.88.0 docker digest to a5c5c4b ([edb3a60](https://gitlab.com/lx-industries/rmcp-actix-web/commit/edb3a60d89583db05bd8e61898c619d8b6bbc393))
* **deps:** update rust:1.88.0 docker digest to af306cf ([38d0324](https://gitlab.com/lx-industries/rmcp-actix-web/commit/38d032423785dfb5bbba215ba697b33bc60eac32))
* **deps:** update rust:1.88.0 docker digest to d8fb475 ([14d22f2](https://gitlab.com/lx-industries/rmcp-actix-web/commit/14d22f225924982ec3e17e7dab912c372eb8a171))

## [0.2.3](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.2...v0.2.3) (2025-07-21)


### Miscellaneous Chores

* **deps:** update rust crate serde_json to v1.0.141 ([0484466](https://gitlab.com/lx-industries/rmcp-actix-web/commit/04844662ec8cdc567637ca1018c79c9a70be0107))

## [0.2.2](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.1...v0.2.2) (2025-07-17)


### Miscellaneous Chores

* **deps:** update node.js to v22.17.1 ([805838e](https://gitlab.com/lx-industries/rmcp-actix-web/commit/805838eb6e8eaa61df5797e286509e3e84510800))
* **deps:** update rust crate rmcp to v0.3.0 ([6b340ee](https://gitlab.com/lx-industries/rmcp-actix-web/commit/6b340ee89466aa0f82510d702ad71db90c64346b))

## [0.2.1](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.2.0...v0.2.1) (2025-07-11)


### Miscellaneous Chores

* **deps:** update node.js to 2c3f34d ([48bd4c6](https://gitlab.com/lx-industries/rmcp-actix-web/commit/48bd4c6a318e5564444491e8eff1614122d76476))
* **deps:** update node.js to v22 ([4f765a2](https://gitlab.com/lx-industries/rmcp-actix-web/commit/4f765a2fd8ea8e1dc167041a23b65453cd9f54d2))
* **deps:** update rust crate tokio to v1.46.1 ([1124982](https://gitlab.com/lx-industries/rmcp-actix-web/commit/11249822ee69d9d69849695aa10293265bc3e6ba))
* **deps:** update rust:1.88.0 docker digest to 5771a3c ([00ec1b4](https://gitlab.com/lx-industries/rmcp-actix-web/commit/00ec1b4bf260bbd589e5be54979a0bd705006147))

## [0.2.0](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.1.0...v0.2.0) (2025-07-07)


### Features

* add Cargo features for selective transport compilation ([7bebcdb](https://gitlab.com/lx-industries/rmcp-actix-web/commit/7bebcdb5b037037d17824df85966ce26c9473538)), closes [#10](https://gitlab.com/lx-industries/rmcp-actix-web/issues/10)
* add framework-level composition APIs aligned with RMCP patterns ([c82e514](https://gitlab.com/lx-industries/rmcp-actix-web/commit/c82e51425c2d7518a4fcff81a85231848785c703)), closes [#11](https://gitlab.com/lx-industries/rmcp-actix-web/issues/11)

## [0.1.0](https://gitlab.com/lx-industries/rmcp-actix-web/compare/v0.0.0...v0.1.0) (2025-07-05)


### Features

* add actix-web MCP transport with GitLab CI pipeline ([c1af689](https://gitlab.com/lx-industries/rmcp-actix-web/commit/c1af689ba42aca9d1bd57a2f3ce5c57351480b0f))
* extract actix-web transport from rust-sdk ([b32bb32](https://gitlab.com/lx-industries/rmcp-actix-web/commit/b32bb32cc55b37ee8658a6614c70b01d2b9a8e7c))


### Bug Fixes

* keywords must have less than 20 characters error when running cargo publish ([01fc85b](https://gitlab.com/lx-industries/rmcp-actix-web/commit/01fc85b953c606a662d5b5560b2e1ee6097b2de2))
