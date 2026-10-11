// Variant 28: deinterleave rows
// A1[k,j] = B[2k,j];  A2[k,j] = B[2k+1,j]

// Baseline 2D grid: threadIdx.x -> column (already coalesced-friendly).
extern "C" __global__ void deinterleave_basic(
    const int* __restrict__ B,
    int* __restrict__ A1,
    int* __restrict__ A2,
    int K,
    int M
) {
    int k = (int)(blockIdx.y * blockDim.y + threadIdx.y);
    int j = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (k >= K || j >= M) {
        return;
    }
    A1[k * M + j] = B[(2 * k) * M + j];
    A2[k * M + j] = B[(2 * k + 1) * M + j];
}

// Deliberately non-coalesced: neighboring threads stride by M in memory.
extern "C" __global__ void deinterleave_naive(
    const int* __restrict__ B,
    int* __restrict__ A1,
    int* __restrict__ A2,
    int K,
    int M
) {
    int j = (int)(blockIdx.y * blockDim.y + threadIdx.y);
    int k = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (k >= K || j >= M) {
        return;
    }
    A1[k * M + j] = B[(2 * k) * M + j];
    A2[k * M + j] = B[(2 * k + 1) * M + j];
}

// Explicit coalesced layout + vectorized int4 loads/stores when M allows.
extern "C" __global__ void deinterleave_coalesced(
    const int* __restrict__ B,
    int* __restrict__ A1,
    int* __restrict__ A2,
    int K,
    int M
) {
    int k = (int)(blockIdx.y * blockDim.y + threadIdx.y);
    int j0 = (int)((blockIdx.x * blockDim.x + threadIdx.x) * 4);
    if (k >= K || j0 >= M) {
        return;
    }

    const int* row_even = B + (2 * k) * M;
    const int* row_odd = B + (2 * k + 1) * M;
    int* out1 = A1 + k * M;
    int* out2 = A2 + k * M;

    if (j0 + 3 < M) {
        int4 e = *reinterpret_cast<const int4*>(row_even + j0);
        int4 o = *reinterpret_cast<const int4*>(row_odd + j0);
        *reinterpret_cast<int4*>(out1 + j0) = e;
        *reinterpret_cast<int4*>(out2 + j0) = o;
    } else {
        for (int j = j0; j < M; ++j) {
            out1[j] = row_even[j];
            out2[j] = row_odd[j];
        }
    }
}

// Same as coalesced, but processes a K-slice starting at k_base (for multi-stream).
extern "C" __global__ void deinterleave_coalesced_offset(
    const int* __restrict__ B,
    int* __restrict__ A1,
    int* __restrict__ A2,
    int K_tile,
    int M,
    int k_base
) {
    int k_local = (int)(blockIdx.y * blockDim.y + threadIdx.y);
    int j0 = (int)((blockIdx.x * blockDim.x + threadIdx.x) * 4);
    if (k_local >= K_tile || j0 >= M) {
        return;
    }
    int k = k_base + k_local;

    const int* row_even = B + (2 * k) * M;
    const int* row_odd = B + (2 * k + 1) * M;
    int* out1 = A1 + k * M;
    int* out2 = A2 + k * M;

    if (j0 + 3 < M) {
        int4 e = *reinterpret_cast<const int4*>(row_even + j0);
        int4 o = *reinterpret_cast<const int4*>(row_odd + j0);
        *reinterpret_cast<int4*>(out1 + j0) = e;
        *reinterpret_cast<int4*>(out2 + j0) = o;
    } else {
        for (int j = j0; j < M; ++j) {
            out1[j] = row_even[j];
            out2[j] = row_odd[j];
        }
    }
}
