#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS interface_in;

void main() {

    vec4 blended = vec4(0.0);

    for (int i = 0; i < 4; i++)
        blended += subpassLoad(interface_in, i);

    fragment_color = blended / 4.0;
}
