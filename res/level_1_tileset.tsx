<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.4" tiledversion="1.4.3" name="GGQ" tilewidth="16" tileheight="16" spacing="1" tilecount="64" columns="8">
 <image source="level_1_tileset.png" width="135" height="135"/>
 <tile id="0">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="1">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="2">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="3">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="foreground" value="true"/>
   <property name="shootable" value="true"/>
   <property name="water" value="true"/>
  </properties>
 </tile>
 <tile id="4">
  <properties>
   <property name="collision_shape" value="triangle_nw"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="5">
  <properties>
   <property name="collision_shape" value="triangle_ne"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="6">
  <properties>
   <property name="collider" value="true"/>
   <property name="collision_shape" value="square"/>
   <property name="contact_damage" value="true"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="7">
  <properties>
   <property name="collider" value="true"/>
   <property name="collision_shape" value="square"/>
   <property name="contact_damage" value="true"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="8">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="9">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="ratchet" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="10">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="ratchet" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="11">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="ratchet" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="12">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="13">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="15">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="entity_class" value="FallingBridge"/>
   <property name="ratchet" value="true"/>
  </properties>
 </tile>
 <tile id="35">
  <properties>
   <property name="collision_shape" value="square"/>
   <property name="ground" value="true"/>
   <property name="shootable" value="true"/>
  </properties>
 </tile>
 <tile id="37">
  <properties>
   <property name="entity_class" value="CheckPoint"/>
  </properties>
 </tile>
 <tile id="38">
  <properties>
   <property name="entity_class" value="SpawnPoint"/>
   <property name="spawned_entity_class" value="FireSprite"/>
  </properties>
 </tile>
 <tile id="39">
  <properties>
   <property name="entity_class" value="SpawnPoint"/>
   <property name="spawned_entity_class" value="FlyingFish"/>
  </properties>
 </tile>
 <tile id="40">
  <properties>
   <property name="animation" value="fire_window"/>
   <property name="animation_duration" value="0.1"/>
   <property name="animation_frame" value="0"/>
  </properties>
 </tile>
 <tile id="41">
  <properties>
   <property name="animation" value="fire_window"/>
   <property name="animation_duration" value="0.1"/>
   <property name="animation_frame" value="1"/>
  </properties>
 </tile>
 <tile id="42">
  <properties>
   <property name="animation" value="fire_window"/>
   <property name="animation_duration" value="0.1"/>
   <property name="animation_frame" value="2"/>
  </properties>
 </tile>
 <tile id="43">
  <properties>
   <property name="animation" value="fire_window"/>
   <property name="animation_duration" value="0.1"/>
   <property name="animation_frame" value="3"/>
  </properties>
 </tile>
 <tile id="44">
  <properties>
   <property name="animation" value="fire_window"/>
   <property name="animation_duration" value="0.1"/>
   <property name="animation_frame" value="4"/>
  </properties>
 </tile>
 <tile id="45">
  <properties>
   <property name="animation" value="fire_window"/>
   <property name="animation_duration" value="0.1"/>
   <property name="animation_frame" value="5"/>
  </properties>
 </tile>
 <tile id="46">
  <properties>
   <property name="entity_class" value="SpawnPoint"/>
   <property name="spawned_entity_class" value="Hoodie"/>
  </properties>
 </tile>
 <tile id="47">
  <properties>
   <property name="entity_class" value="SpawnPoint"/>
   <property name="spawned_entity_class" value="Bat"/>
  </properties>
 </tile>
 <tile id="48">
  <properties>
   <property name="arena_height" value="10"/>
   <property name="arena_width" value="17"/>
   <property name="entity_class" value="SpawnPoint"/>
   <property name="spawned_entity_class" value="BossFish"/>
   <property name="water_height" value="4"/>
  </properties>
 </tile>
 <tile id="50">
  <properties>
   <property name="entity_class" value="BossFightTrigger"/>
  </properties>
 </tile>
 <tile id="51">
  <properties>
   <property name="collision_shape" value="Square"/>
   <property name="entity_class" value="PowerUp"/>
   <property name="powerup_type" value="vial"/>
  </properties>
 </tile>
 <tile id="52">
  <properties>
   <property name="collision_shape" value="Square"/>
   <property name="entity_class" value="PowerUp"/>
   <property name="powerup_type" value="heart"/>
  </properties>
 </tile>
</tileset>
