#[macro_export]
macro_rules! parse_line {
    ($binding:literal: $name:literal in $type:tt: [$buffer_type:ty; $array_count:literal]) => {
        MixturePart {
            binding: $binding,
            name: String::from($name),
            shader_type: ShaderType::$type,
            is_dynamic: false,
            array_size: $array_count,
            type_info: PartType::Uniform(std::mem::size_of::<$buffer_type>()),
        }
    };
    ($binding:literal: $name:literal in $type:tt: [dynamic $buffer_type:ty]) => {
        MixturePart {
            binding: $binding,
            name: String::from($name),
            shader_type: ShaderType::$type,
            is_dynamic: true,
            array_size: 1,
            type_info: PartType::Uniform(std::mem::size_of::<$buffer_type>()),
        }
    };
    ($binding:literal: $name:literal in $type:tt: sampler) => {
        MixturePart {
            binding: $binding,
            name: String::from($name),
            shader_type: ShaderType::$type,
            is_dynamic: false,
            array_size: 1,
            type_info: PartType::Sampler,
        }
    };
    ($binding:literal: $name:literal in $type:tt: $buffer_type:ty) => {
        MixturePart {
            binding: $binding,
            name: String::from($name),
            shader_type: ShaderType::$type,
            is_dynamic: false,
            array_size: 1,
            type_info: PartType::Uniform(std::mem::size_of::<$buffer_type>()),
        }
    };
}

/// This macro is probably a horrific mess
///
/// The basic idea is and please not that it is very likely that corner cases are not properly tested
/// mixture![
///     $line,
/// ]
/// Where each line has the following form: (please not that the braces {} correspond to the corresponding
/// variable and are not actually matched)
/// {binding}: "{name}" in {shader_type}: $descriptor_type
/// Where descriptor type is either
///   - A Plain Type: ViewProjectionMatrix
///   - A Plain Type but as a dynamic uniform buffer: [dynamic Material] (the [] braces are actually needed)
///   - An Array type of any plain type: [DirectionalLight; 20]
///   - Just the word: 'sampler'
///
/// ** Examples
/// - 0: "lights" in fragment: [Light; 20]
/// - 1: "camera" in vertex: ShaderCamera
/// - 2: "material" in fragment: [dynamic Material]
/// - 3: "albedo" in fragment: sampler
#[macro_export]
macro_rules! mixture {
    [$($binding:literal: $name:literal in $type:ident: $rest:tt),*] => {
        vec![$(parse_line!($binding: $name in $type: $rest),)*]
    };
}
