searchState.loadedDescShard("ecolor", 0, "Color conversions and types.\nThis format is used for space-efficient color …\n3 hexadecimal digits, one for each of the r, g, b channels\n4 hexadecimal digits, one for each of the r, g, b, a …\n6 hexadecimal digits, two for each of the r, g, b channels\n8 hexadecimal digits, one for each of the r, g, b, a …\nA wrapper around Color32 that converts to and from a …\nHue, saturation, value, alpha. All in the range [0, 1]. No …\nLike Hsva but with the <code>v</code> value (brightness) being gamma …\nAn ugly color that is planned to be replaced before making …\n0-1 linear space <code>RGBA</code> color with premultiplied alpha.\nalpha 0-1. A negative value signifies an additive color …\nalpha 0-1. A negative value signifies an additive color …\nReturns an additive version of self\nReturn an additive version of this color (alpha = 0)\nRetrieves the inner <code>Color32</code>\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nTransparent black\nParses a color from a hex string.\nFrom <code>sRGBA</code> with premultiplied alpha.\nFrom linear RGBA with premultiplied alpha\nFrom <code>sRGBA</code> WITHOUT premultiplied alpha.\nFrom linear RGBA without premultiplied alpha\nFrom <code>sRGBA</code> with premultiplied alpha\nFrom <code>sRGBA</code> without premultiplied alpha\nParses a string as a hex color without the leading <code>#</code> …\nTransparent white\nlinear [0, 1] -&gt; gamma [0, 1] (not clamped). Works for …\nMultiply with 0.5 to make color half as opaque, …\nlinear [0, 1] -&gt; gamma [0, 255] (clamped). Values outside …\nhue 0-1\nhue 0-1\nAll ranges in 0-1, rgb is linear.\nHow perceptually intense (bright) is the color?\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nIs the alpha=0 ?\nIs the alpha=0 ?\nLerp this color towards <code>other</code> by <code>t</code> in gamma space.\ngamma [0, 255] -&gt; linear [0, 1].\nlinear [0, 255] -&gt; linear [0, 1]. Useful for alpha-channel.\ngamma [0, 1] -&gt; linear [0, 1] (not clamped). Works for …\nMultiply with 0.5 to make color half as opaque in linear …\nlinear [0, 1] -&gt; linear [0, 255] (clamped). Useful for …\nMultiply with e.g. 0.5 to make us half transparent\nAll ranges in 0-1, rgb is linear.\nsaturation 0-1\nsaturation 0-1\nCheap and ugly. Made for graying out disabled <code>Ui</code>s.\nPremultiplied RGBA\nPremultiplied RGBA\nFormats the color as a hex string.\nConverts to floating point values in the range 0-1 without …\nReturns an opaque version of self\nReturns an opaque version of self\nTo linear space rgba in 0-1 range.\nunmultiply the alpha\nTo gamma-space 0-255.\nunmultiply the alpha\nPremultiplied RGBA\nPremultiplied RGBA\nvalue 0-1, in gamma-space (~perceptually even)\nvalue 0-1")