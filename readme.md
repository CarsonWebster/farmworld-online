# 🌱 FarmWorld Online (Graal-Inspired Social Sandbox)

## 🎮 Game Idea
A lighthearted, cross-platform **social farming MMO** inspired by *Graal Online Classic* and *Animal Crossing*.  
Players live in a shared persistent world where they can:
- **Farm**: Plant, water, and harvest crops.
- **Socialize**: Chat, emote, join guilds, and visit friends’ farms.
- **Decorate**: Customize farms and houses with furniture and cosmetics.
- **Explore**: Walk around a tile-based world with towns, shops, and seasonal events.

The focus is on **social activities, creativity, and community-driven fun** rather than combat.

---

## 🚀 MVP (Prototype with Godot High-Level Multiplayer)

The MVP will be built in **Godot** using the **High-Level Multiplayer API**.  
This allows rapid prototyping of the core loop before migrating to a Rust backend.

### MVP Features
- **Multiplayer movement sync**: Multiple players walking around the same map.
- **Chat system**: Text bubbles above players.
- **Basic farming**: Plant a seed, wait, harvest a crop.
- **Simple persistence**: Temporary (resets on restart).
- **Basic art**: Simple sprites made in Aseprite.

### MVP TODO List
- [ ] Set up Godot project with tile-based map.
- [ ] Implement player movement + sync via High-Level API.
- [ ] Add chat bubbles above players.
- [ ] Create a farm plot node where players can plant/harvest.
- [ ] Add simple crop growth timer (client-side for now).
- [ ] Add placeholder sprites for player, crops, and tiles.
- [ ] Package for PC + Mobile builds.

---

## 🌍 Full Game Plan (Rust Backend + Bevy Simulation)

Once the MVP is working, migrate to a **Rust backend** for scalability, persistence, and security.

### 🏗️ Architecture
```
+-------------------+        +-------------------+
|   Godot Client    | <----> |   Gateway Server  |  (axum + WebSockets)
| (PC /Mobile/Web)  |        |  (Auth, Routing)  |
+-------------------+        +-------------------+
							 |
							 v
					+-------------------+
					|   Game Logic Svc  |  (Bevy ECS)
					|  (Authoritative)  |
					+-------------------+
					  |
+---------------------+---------------------+
|                                           |
v                                           v
+-------------------+                         +-------------------+
|   Database (SQL)  |                         |   Cache (Redis)   |
| Postgres (truth)  |                         | Timers, sessions  |
+-------------------+                         +-------------------+
```
### 🔄 Example Flow
1. **Login**: Client logs in via OAuth → Gateway verifies → issues JWT.
2. **Connect**: Client opens WebSocket → Gateway authenticates → routes to Game Logic.
3. **Simulate**: Game Logic (Bevy ECS) simulates world → updates Postgres (truth) + Redis (fast).
4. **Update**: Server sends authoritative updates → Client renders.
5. **Persistence**: Postgres stores farms, inventories, accounts. Redis stores sessions, timers.

---

## 📦 Rust Crates
- **Networking**: `axum`, `tokio`, `tokio-tungstenite`, `serde`
- **Game Logic**: `bevy` (ECS)
- **Database**: `diesel` (ORM, type-safe), `postgres` driver
- **Cache**: `redis`
- **Auth**: `oauth2`, `argon2`, `jsonwebtoken`
- **Utilities**: `uuid`, `chrono`

---

## 📡 Example Client ↔ Server Messages

### Client → Server
```json
{
  "action": "plant_crop",
  "crop_type": "wheat",
  "x": 10,
  "y": 5
}
```
### Server → Client
```json
{
  "event": "crop_planted",
  "entity_id": "123e4567-e89b-12d3-a456-426614174000",
  "crop_type": "wheat",
  "x": 10,
  "y": 5,
}
```

## 🔍 Message Validation

- Client messages are parsed with serde_json.
- Gateway verifies JWT → attaches user_id.
- Game Logic checks:
	- Does player own the seed?
	- Is the tile empty?
	- Is the action allowed in this zone?
- If valid → update ECS world + persist to DB.
- If invalid → send error back to client.

## 🧩 Example Bevy ECS Setup
### Entity
- CropEntity
### Components

```rust
use bevy::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Component)]
pub struct Crop {
	pub crop_type: String,
	pub planted_at: DateTime<Utc>,
	pub growth_duration: i64, // seconds
	pub owner_id: uuid::Uuid,
}
```

### System
```rust
use bevy::prelude::*;
use chrono::Utc;
use diesel::prelude::*;
use crate::schema::crops;

pub fn crop_growth_system(
	mut query: Query<(Entity, &Crop)>,
	mut commands: Commands,
	pool: Res<DbPool>, // Diesel connection pool
) {
	let now = Utc::now();
	for (entity, crop) in query.iter() {
		let ready_time = crop.planted_at + chrono::Duration::seconds(crop.growth_duration);
		if now >= ready_time {
			// Harvest ready
			println!("Crop ready: {:?}", crop.crop_type);

			// Update Postgres
			let conn = &mut pool.get().unwrap();
			diesel::delete(crops::table.find(entity.id())).execute(conn).unwrap();

			// Remove entity from ECS
			commands.entity(entity).despawn();
		}
	}
}
```

### Example Postgres schema
```SQL
CREATE TABLE crops (
  id UUID PRIMARY KEY,
  owner_id UUID REFERENCES users(id),
  crop_type TEXT NOT NULL,
  planted_at TIMESTAMP NOT NULL,
  growth_duration INT NOT NULL
);
```

### Scaling Strategy
- Gateways: Stateless, scale horizontally behind load balancer.
- Game Logic: Shard by zone/farm cluster. Each shard runs its own ECS loop.
- Database: Postgres with connection pooling (pgbouncer). Scale read replicas if needed.
- Cache: Redis for sessions, timers, pub/sub between servers.
- Containers: Docker Compose for dev, Kubernetes for production.
