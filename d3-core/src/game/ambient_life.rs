use std::{collections::HashMap, io::{BufRead, BufReader, BufWriter, Read, Seek, Write}, rc::Rc};

use tinyrand::Rand;

use crate::{create_rng, string::D3String};

use super::{context::GameContext, object::ObjectTypeDef};

use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt, BigEndian};

const VERSION: u32 = 1;

pub enum AmbientLifeIndexType {
    Type,
    Total,
    Max,
    Min,
    Flags
}

pub enum AmbientLifeValue {
    Type(Option<ObjectTypeDef>),
    Total(u8),
    Max(u8),
    Min(u8),
    Flags(u8)
}

impl AmbientLifeValue {
    pub fn from_field_id<T> (id: usize, value: T) -> Result<AmbientLifeValue>
    where i32: From<T>, u8: From<T>, ObjectTypeDef: From<T> {
        match id {
            0 => Ok(AmbientLifeValue::Type(Some(ObjectTypeDef::from(value).clone()))),
            1 => Ok(AmbientLifeValue::Total(u8::from(value))),
            2 => Ok(AmbientLifeValue::Max(u8::from(value))),
            3 => Ok(AmbientLifeValue::Min(u8::from(value))),
            4 => Ok(AmbientLifeValue::Flags(u8::from(value))),
            _ => Err(anyhow!("invalid field id"))
        }
    }
}

pub const CHECK_INTERVAL_MIN:f32 = 5.0;
pub const CHECK_INTERVAL_MAX:f32 = 10.0;

pub const MAX_AL_TYPES: usize = 6;

pub struct AmbientLife {
    // These are settable by the editor
    state_type: [Option<ObjectTypeDef>; MAX_AL_TYPES],
    state_total: [u8; MAX_AL_TYPES],
    state_max: [u8; MAX_AL_TYPES],
    state_min: [u8; MAX_AL_TYPES],
    state_flags: [u8; MAX_AL_TYPES],

    // internal
    state_current_num: [u8; MAX_AL_TYPES],

    // Don't save these
    state_next_size: [u8; MAX_AL_TYPES],
    state_next_dotime: [f32; MAX_AL_TYPES],
}

impl AmbientLife {
    fn do_frame(&mut self) {

    }

    fn compute_next_size(&mut self, i: usize) {
        let diff = self.state_max[i].wrapping_sub(self.state_min[i]) as i8;
        
        if diff > 0 {
            let offset = (create_rng().next_u32() % diff as u32) as i8;
            self.state_next_size[i] = self.state_min[i] + offset as u8;
        }
        else {
            self.state_next_size[i] = self.state_max[i];
        }
    }

    fn init_for_level(&mut self, context: &Rc<GameContext>) {
        for i in 0..MAX_AL_TYPES {
            self.compute_next_size(i);
            self.state_current_num[i] = 0;
            self.state_next_dotime[i] = context.gametime();
        }

        self.do_frame();
    }

    pub fn get_value(&self, index: usize, field: AmbientLifeIndexType) -> AmbientLifeValue {
        match field {
            AmbientLifeIndexType::Type => AmbientLifeValue::Type(self.state_type[index].clone()),
            AmbientLifeIndexType::Total => AmbientLifeValue::Total(self.state_total[index]),
            AmbientLifeIndexType::Max => AmbientLifeValue::Max(self.state_max[index]),
            AmbientLifeIndexType::Min => AmbientLifeValue::Min(self.state_min[index]),
            AmbientLifeIndexType::Flags => AmbientLifeValue::Flags(self.state_flags[index])
        }
    }

    pub fn set_value(&mut self, index: usize, value: AmbientLifeValue) {
        match value {
            AmbientLifeValue::Type(v) => self.state_type[index] = v,
            AmbientLifeValue::Total(v) => self.state_total[index] = v,
            AmbientLifeValue::Max(v) => self.state_max[index] = v,
            AmbientLifeValue::Min(v) => self.state_min[index] = v,
            AmbientLifeValue::Flags(v) => self.state_flags[index] = v,
        }

        if self.state_max[index] > self.state_total[index] {
            self.state_max[index] = self.state_total[index];
        }

        if self.state_min[index] > self.state_max[index] {
            self.state_min[index] = self.state_max[index];
        }

        self.compute_next_size(index);
    }

    pub fn reset(&mut self) {
        for i in 0..MAX_AL_TYPES {
            self.state_type[i] = None;
            self.state_total[i] = 0;
            self.state_current_num[i] = 0;
            self.state_flags[i] = 0;
            self.state_min[i] = 0;
            self.state_max[i] = 0;
            self.state_next_dotime[i] = 0.0;
            self.state_next_size[i] = 0;
        }
    }

    pub fn save_data<T: Write>(&self, writer: &mut BufWriter<T>) {
        writer.write_u32::<LittleEndian>(VERSION).unwrap();

        for i in 0..MAX_AL_TYPES {
            let obj_type = &self.state_type[i];

            if obj_type.is_some() {
                let obj_type = obj_type.as_ref().unwrap();

                writer.write_i16::<LittleEndian>(obj_type.name.len() as i16);

                for j in 0..obj_type.name.len() {
                    writer.write_u8(obj_type.name[j]);
                }
            }
            else {
                writer.write_i16::<LittleEndian>(1);
                writer.write_u8(b'\0');
            }

            writer.write_u8(self.state_total[i]);
            writer.write_u8(self.state_flags[i]);
            writer.write_u8(self.state_min[i]);
            writer.write_u8(self.state_max[i]);
            writer.write_u8(self.state_next_size[i]);
            writer.write_f32::<LittleEndian>(self.state_next_dotime[i]);
        }

        for i in 0..MAX_AL_TYPES {
            let cur_num = self.state_current_num[i];
            writer.write_u8(cur_num);

            for j in 0..cur_num {
                /* XXX: We just write empty 32-bit handles */
                // D3 doesn't seem to do anything with them anyways
                writer.write_u32::<LittleEndian>(0);
            }
        }
    }

    pub fn load_data<T: Read + Seek>(&mut self, reader: &mut BufReader<T>, type_hashmap: &HashMap<String, ObjectTypeDef>) {
        let version = reader.read_u32::<LittleEndian>().unwrap();

        if version < 1 {
            panic!("refuse to parse outdated info");
        }

        for i in 0..MAX_AL_TYPES {
            let len = reader.read_i16::<LittleEndian>().unwrap();

            let mut name = vec![0u8; len as usize];

            for j in 0..len {
                name.push(reader.read_u8().unwrap());
            }
            
            let name = D3String::from_slice(name.as_slice());

            let object_type = type_hashmap[&name.to_string().unwrap()].clone();
            self.state_type[i] = Some(object_type);
            self.state_total[i] = reader.read_u8().unwrap();
            self.state_flags[i] = reader.read_u8().unwrap();
            self.state_min[i] = reader.read_u8().unwrap();
            self.state_max[i] = reader.read_u8().unwrap();
            self.state_next_size[i] = reader.read_u8().unwrap();
            self.state_next_dotime[i] = reader.read_f32::<LittleEndian>().unwrap();
        }

        for i in 0..MAX_AL_TYPES {
            self.state_current_num[i] = reader.read_u8().unwrap();
            let _ = reader.seek(std::io::SeekFrom::Current((self.state_current_num[i] * 4) as i64));
        }
    }
}