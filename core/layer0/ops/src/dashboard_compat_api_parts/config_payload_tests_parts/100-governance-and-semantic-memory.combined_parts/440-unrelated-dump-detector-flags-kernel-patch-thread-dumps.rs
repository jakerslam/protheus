fn unrelated_dump_detector_flags_kernel_patch_thread_dumps() {
    let dump = "[PATCH v2 1/2] drm/msm/dpu: allow encoder to be created with empty dpu_crtc
[Date Prev][Date Next][Thread Prev][Thread Next][Date Index][Thread Index]
To: Rob Clark <robdclark@example.com>
Subject: [PATCH v2 1/2] drm/msm/dpu: allow encoder to be created with empty dpu_crtc
From: Jessica Zhang <quic_jesszhan@example.com>
In-reply-to: 20230901202143.16356-1-quic_jesszhan@quicinc.com
Signed-off-by: Jessica Zhang <quic_jesszhan@example.com>
diff --git a/drivers/gpu/drm/msm/disp/dpu1/dpu_encoder.c b/drivers/gpu/drm/msm/disp/dpu1/dpu_encoder.c";
    assert!(response_is_unrelated_context_dump(
        "So do you think the system is getting more capable?",
        dump
    ));
}

#[test]
