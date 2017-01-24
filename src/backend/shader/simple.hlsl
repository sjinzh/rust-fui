struct VsOutput {
    float4 pos: SV_Position;
    float4 color: COLOR;
};

VsOutput Vertex(float2 pos : a_Pos, float4 color : a_Color) {
    VsOutput output = {
        float4(pos, 0.0, 1.0),
        color,
    };
    return output;
}

float4 Pixel(VsOutput pin) : SV_Target {
    return float4(pin.color);
}
