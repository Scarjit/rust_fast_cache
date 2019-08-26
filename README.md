# Rust Fast Cache

Rust fast cache is an memory cached, disk cache for (String, Vec\<u8>) entries.
Each entry is first cached in memory only, and only writen to disk,
if the cache runs out of memory and the item is seldom used.
If the disk cache is also full, it will remove the least desired items, either by last access or access count.

Internally it uses a Hashmap with XxHash64 as hashing algorithm. (It is NOT DoS resistant.)

Rust Fast Cache is threadsafe, although it uses RwLocks, so simultaneous write is not possible.

This project is designed as the caching part for Rust Lan Cache and will be modified to match it's needs.

### TODO
 - More tests
 - Prevent unneeded disk access (when disk cache runs full)


### Credits
 - CLion by Jetbrains <3