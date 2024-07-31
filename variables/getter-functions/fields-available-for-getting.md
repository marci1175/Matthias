---
description: >-
  This page lists most (as it is always changing) fields which have a getter
  function.
---

# 🙏 Fields available for getting

The links included will point to struct definitions in the source code, all the entries which have no tags or fields which have `#[table(save)]` (It does not matter if there is `#[serde(skip)]` there) can be accessed through the API system.

* [Application](../../src/app/backend.rs#L50)
* [Client](../../src/app/backend.rs#L443)
* [ClientConnection](../../src/app/backend.rs#L1156)
* [UserInformation](../../src/app/backend.rs#L2091)

<pre class="language-rust"><code class="lang-rust"><strong>pub struct Application {
</strong>    #[serde(skip)]
    pub lua: Arc&#x3C;Lua>, //This entry cannot be accessed
    
    pub login_username: String, //This entry can be accessed
    
    #[table(save)]
    #[serde(skip)] //This entry can be accessed
    pub server_connected_clients_profile: Arc&#x3C;DashMap&#x3C;String, ClientProfile>>,
    
    . . .
}
</code></pre>

To access said entries please refer to [this page](./).
